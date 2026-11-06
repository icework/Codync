//! Decisions-based Codex routing. Only host code calls the API; persisted routes contain no prompts or keys.
mod profiles;
mod quota;
#[cfg(test)]
mod tests;
mod transport;

use crate::{
    hub::Hub,
    store::{BotConfig, Store},
};
use anyhow::{Result, bail};
use profiles::{POLICY, Profile};
pub use quota::allows_fast;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub profile: Option<Profile>,
    pub fast: bool,
    /// A fixed category, never a provider's response or the user's request.
    pub outcome: Outcome,
}

/// Only these fixed categories may reach persistence or telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Selected,
    MissingApiKey,
    CredentialUnavailable,
    ModelsUnavailable,
    ClientError,
    NetworkError,
    ApiRejected,
    InvalidResponse,
    Timeout,
}
impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::MissingApiKey => "missing_api_key",
            Self::CredentialUnavailable => "credential_unavailable",
            Self::ModelsUnavailable => "models_unavailable",
            Self::ClientError => "client_error",
            Self::NetworkError => "network_error",
            Self::ApiRejected => "api_rejected",
            Self::InvalidResponse => "invalid_response",
            Self::Timeout => "timeout",
        }
    }
}

pub fn enabled(cfg: &BotConfig) -> bool {
    cfg.smart_routing && cfg.backend == "codex" && !cfg.is_group()
}

/// A logical boundary measured from the end of the previous turn, independent of context size.
pub fn idle_boundary(cfg: &BotConfig, last_turn: i64, now: i64) -> bool {
    enabled(cfg) && last_turn > 0 && now.saturating_sub(last_turn) >= 30 * 60 * 1000
}

pub fn key(bot: &str, sid: &str) -> String {
    format!("routing.{bot}.{sid}")
}

pub fn saved(store: &Store, cfg: &BotConfig, sid: &str) -> Result<Option<Route>> {
    if !enabled(cfg) {
        return Ok(None);
    }
    store.kv_read(&key(&cfg.id, sid))?.map(|s| serde_json::from_str(&s).map_err(Into::into)).transpose()
}

impl Route {
    pub fn config(&self, cfg: &BotConfig) -> BotConfig {
        let mut selected = cfg.clone();
        if let Some(profile) = self.profile {
            selected.model = Some(profile.model().into());
            selected.model_options.clear();
            selected.model_options.insert("reasoning_effort".into(), profile.effort().into());
        }
        selected.fast_mode = self.fast;
        selected
    }

    pub fn summary(&self) -> String {
        self.profile.map_or_else(
            || {
                format!(
                    "Smart routing unavailable ({}). Using the bot's saved model at standard speed.",
                    self.outcome.label()
                )
            },
            |p| {
                format!(
                    "Smart routing: {} · {}{}",
                    p.model(),
                    p.effort(),
                    if self.fast { " · Fast" } else { " · Standard" }
                )
            },
        )
    }
}

fn choices(session: &Value, fast: bool) -> Vec<(String, Profile, bool)> {
    let models = crate::agent::auth::model_ids(session);
    Profile::ALL
        .into_iter()
        .filter(|p| models.iter().any(|m| m == p.model()))
        .flat_map(|p| {
            [false, true]
                .into_iter()
                .filter(move |&f| !f || fast)
                .map(move |f| (format!("{}-{}", p.id(), if f { "fast" } else { "standard" }), p, f))
        })
        .collect()
}

fn request(prompt: &str, instructions: &str, offered: &[(String, Profile, bool)], fast: bool) -> Value {
    // Bound classification cost; no tool results, files, private memory or chat history go to this API.
    let data = json!({"request": prompt.chars().take(16000).collect::<String>(),
        "standing_instructions": instructions.chars().take(2000).collect::<String>(), "quota_allows_fast": fast});
    json!({"model": "gpt-6-luna", "input": data.to_string(), "questions": [{
        "type": "choice", "name": "route", "instructions": POLICY,
        "choices": offered.iter().map(|(id,p,f)| json!({"value": id,
            "description": format!("{} Speed: {}.", p.description(), if *f { "Fast (higher usage)" } else { "standard" })})).collect::<Vec<_>>()
    }]})
}

fn answer(value: &Value, offered: &[(String, Profile, bool)]) -> Result<Route, Outcome> {
    let answers = value["answers"].as_array().ok_or(Outcome::InvalidResponse)?;
    if answers.len() != 1 {
        return Err(Outcome::InvalidResponse);
    }
    let a = &answers[0];
    if a["name"] != "route" || a["type"] != "choice" {
        return Err(Outcome::InvalidResponse);
    }
    let Some((_, profile, fast)) = offered.iter().find(|(id, _, _)| a["choice"].as_str() == Some(id)) else {
        return Err(Outcome::InvalidResponse);
    };
    Ok(Route { profile: Some(*profile), fast: *fast, outcome: Outcome::Selected })
}

async fn api_key(hub: &Arc<Hub>) -> Result<zeroize::Zeroizing<String>, Outcome> {
    // Reuse the encrypted call key. No routing-only secret is inherited by agent processes.
    if hub.store.kv_read("voice").map_err(|_| Outcome::CredentialUnavailable)?.is_none() {
        return Err(Outcome::MissingApiKey);
    }
    crate::market::vault::unlock(hub.clone()).await.map_err(|_| Outcome::CredentialUnavailable)?;
    crate::voice::key(&hub.store, crate::voice::Provider::OpenAi)
        .map(zeroize::Zeroizing::new)
        .map_err(|_| Outcome::MissingApiKey)
}

async fn classify(hub: &Arc<Hub>, cfg: &BotConfig, prompt: &str, session: &Value) -> Result<Route, Outcome> {
    let fast = allows_fast(hub, cfg) && crate::agent::config::fast_mode_config(session).is_some();
    let offered = choices(session, fast);
    if offered.len() < 2 {
        return Err(Outcome::ModelsUnavailable);
    }
    let key = api_key(hub).await?;
    let value = transport::send(key.as_str(), &request(prompt, &cfg.description, &offered, fast)).await?;
    answer(&value, &offered)
}

pub async fn choose(hub: &Arc<Hub>, cfg: &BotConfig, prompt: &str, session: &Value) -> Route {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(TIMEOUT, classify(hub, cfg, prompt, session)).await;
    let mut route = match result {
        Ok(Ok(route)) => route,
        Ok(Err(e)) => Route { profile: None, fast: false, outcome: e },
        Err(_) => Route { profile: None, fast: false, outcome: Outcome::Timeout },
    };
    route.fast &= allows_fast(hub, cfg);
    let properties = json!({"outcome": route.outcome, "profile": route.profile.map(Profile::id),
        "fast": route.fast, "duration_ms": started.elapsed().as_millis()});
    crate::analytics::capture(
        hub,
        crate::analytics::Event::SessionRouted,
        properties.as_object().cloned().unwrap_or_default(),
    );
    route
}

/// A successful RPC alone is not evidence that the adapter applied both requested settings.
pub fn verify(session: &Value, route: &Route) -> Result<()> {
    let Some(profile) = route.profile else { return Ok(()) };
    let options =
        session["configOptions"].as_array().ok_or_else(|| anyhow::anyhow!("Missing route acknowledgement"))?;
    let current = |id: &str| options.iter().find(|o| o["id"] == id).and_then(|o| o["currentValue"].as_str());
    let model = crate::agent::auth::model_config(session).and_then(|c| c["currentValue"].as_str());
    if model != Some(profile.model()) || current("reasoning_effort") != Some(profile.effort()) {
        bail!("The agent did not acknowledge the selected model and effort; no prompt was sent");
    }
    Ok(())
}
