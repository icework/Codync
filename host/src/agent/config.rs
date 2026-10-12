//! A session's config options (ACP `configOptions`): the model list and Fast mode.

use serde_json::{Value, json};

/// Fast mode's option IDs: Claude's adapter (`fast`) and Codex's (`fast-mode`).
const FAST_MODE_IDS: [&str; 2] = ["fast", "fast-mode"];

/// IDs are agent-defined: use the category, with legacy conventional IDs as a fallback.
pub fn model_config(session: &Value) -> Option<&Value> {
    let options = session["configOptions"].as_array()?;
    let is_select = |v: &&Value| v["type"] == "select" || v.get("type").is_none();
    options
        .iter()
        .filter(is_select)
        .find(|v| v["category"] == "model")
        .or_else(|| options.iter().filter(is_select).find(|v| v["id"] == "model" || v["id"] == "models"))
}

/// The Fast mode option. Agents offer it only while the session's model supports it.
pub fn fast_mode_config(session: &Value) -> Option<&Value> {
    session["configOptions"].as_array()?.iter().find(|v| v["id"].as_str().is_some_and(|id| FAST_MODE_IDS.contains(&id)))
}

/// Whether the Fast mode option is on.
pub fn fast_mode_on(config: &Value) -> bool {
    config["currentValue"] == true || config["currentValue"] == "on"
}

/// Why the agent turned Fast mode off by itself, when it says: Claude's adapter appends it to
/// the option's description ("Faster responses on supported models — requires extra usage…").
pub fn fast_mode_refusal(config: &Value) -> Option<String> {
    let description = config["description"].as_str()?;
    let (_, why) = description.split_once(" — ")?;
    text(&json!(why))
}

/// What a bot's agent says about Fast mode (wire `fastModeSupport` and `fastModeIssue`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FastModeSupport {
    /// No session has listed its options yet.
    #[default]
    Unknown,
    Offered,
    /// The session's model doesn't offer it.
    Unsupported,
    /// The agent turned it off after the bot asked for it (Claude: the account can't use it),
    /// with its reason when it gave one. Asked again only once the user changes the setting.
    Refused(Option<String>),
}

impl FastModeSupport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Offered => "offered",
            Self::Unsupported => "unsupported",
            Self::Refused(_) => "refused",
        }
    }

    pub fn issue(&self) -> Option<&str> {
        match self {
            Self::Refused(why) => why.as_deref(),
            _ => None,
        }
    }

    /// Kept across host restarts (`fast-mode.<bot>`), so clients can offer the switch before
    /// the first turn. A refusal isn't kept: the next host asks again.
    pub fn stored(&self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Offered | Self::Refused(_) => Some("offered"),
            Self::Unsupported => Some("unsupported"),
        }
    }

    pub fn from_stored(v: Option<&str>) -> Self {
        match v {
            Some("offered") => Self::Offered,
            Some("unsupported") => Self::Unsupported,
            _ => Self::Unknown,
        }
    }
}

/// The value that turns `config` on or off: a boolean, or the `on` / `off` select entries.
/// `None` when it already is.
pub fn fast_mode_change(config: &Value, on: bool) -> Option<Value> {
    let value = if config["type"] == "boolean" { json!(on) } else { json!(if on { "on" } else { "off" }) };
    (config["currentValue"] != value).then_some(value)
}

/// The models a session offers, its current one, and whether it offers Fast mode.
pub fn catalog(session: &Value) -> Value {
    fn collect(options: &Value, out: &mut Vec<Value>) {
        for option in options.as_array().into_iter().flatten() {
            if option["options"].is_array() {
                collect(&option["options"], out);
            } else if let Some(id) = text(&option["value"])
                && !out.iter().any(|v| v["id"] == id)
            {
                out.push(json!({"id": id, "name": text(&option["name"]).unwrap_or(id),
                    "description": text(&option["description"])}));
            }
        }
    }
    let mut models = Vec::new();
    let current = if let Some(config) = model_config(session) {
        collect(&config["options"], &mut models);
        text(&config["currentValue"])
    } else {
        for model in session["models"]["availableModels"].as_array().into_iter().flatten() {
            if let Some(id) = text(&model["modelId"])
                && !models.iter().any(|v| v["id"] == id)
            {
                models.push(json!({"id": id, "name": text(&model["name"]).unwrap_or(id),
                    "description": text(&model["description"])}));
            }
        }
        text(&session["models"]["currentModelId"])
    };
    json!({"models": models, "currentModelId": current, "fastMode": fast_mode_config(session).is_some()})
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_reads_grouped_config_and_preserves_agent_ids() {
        let session = json!({"configOptions": [{
            "id": "provider-model", "category": "model", "type": "select", "currentValue": "m2",
            "options": [{"group": "a", "options": [
                {"value": "m1", "name": "First"}, {"value": "m2", "name": "Second"},
                {"value": "m1", "name": "Duplicate"}
            ]}]
        }], "models": {"availableModels": [{"modelId": "legacy", "name": "Legacy"}]}});
        assert_eq!(model_config(&session).unwrap()["id"], "provider-model");
        let result = catalog(&session);
        assert_eq!(result["currentModelId"], "m2");
        assert_eq!(result["models"].as_array().unwrap().len(), 2);
        assert_eq!(result["models"][0]["id"], "m1");
        assert_eq!(result["models"][1]["name"], "Second");
    }

    #[test]
    fn catalog_reads_legacy_and_missing_lists() {
        let result = catalog(&json!({"models": {
            "currentModelId": "a", "availableModels": [
                {"modelId": "a", "name": "Agent model", "description": "Available here"},
                {"name": "No ID"}
            ]
        }}));
        assert_eq!(result["models"].as_array().unwrap().len(), 1);
        assert_eq!(result["models"][0]["description"], "Available here");
        assert_eq!(catalog(&json!({})), json!({"models": [], "currentModelId": null, "fastMode": false}));
    }

    #[test]
    fn model_config_prefers_category_and_ignores_unknown_types() {
        let session = json!({"configOptions": [
            {"id": "model", "type": "boolean"},
            {"id": "models", "type": "select"},
            {"id": "engine", "category": "model", "type": "select"}
        ]});
        assert_eq!(model_config(&session).unwrap()["id"], "engine");
        assert!(model_config(&json!({"configOptions": [{"id": "model", "type": "boolean"}]})).is_none());
    }

    #[test]
    fn catalog_reports_claude_and_codex_fast_mode() {
        for id in ["fast", "fast-mode"] {
            let session = json!({"configOptions": [
                {"id": "model", "category": "model", "type": "select", "options": []},
                {"id": id, "category": "model_config", "type": "select", "currentValue": "off"}
            ]});
            assert_eq!(fast_mode_config(&session).unwrap()["id"], id);
            assert_eq!(catalog(&session)["fastMode"], true);
        }
        let other = json!({"configOptions": [{"id": "effort", "category": "thought_level", "type": "select"}]});
        assert!(fast_mode_config(&other).is_none());
    }

    #[test]
    fn fast_mode_refusal_reads_the_reason_from_the_description() {
        let refused = json!({"id": "fast", "currentValue": "off",
            "description": "Faster responses on supported models — requires extra usage to be enabled for this account"});
        assert_eq!(fast_mode_refusal(&refused).as_deref(), Some("requires extra usage to be enabled for this account"));
        assert!(!fast_mode_on(&refused));
        assert!(fast_mode_refusal(&json!({"description": "1.5x speed, increased usage"})).is_none());
        assert!(fast_mode_on(&json!({"currentValue": true})));
    }

    #[test]
    fn fast_mode_support_keeps_only_what_the_agent_offers_across_restarts() {
        let refused = FastModeSupport::Refused(Some("no extra usage".into()));
        assert_eq!(refused.as_str(), "refused");
        assert_eq!(refused.issue(), Some("no extra usage"));
        assert_eq!(FastModeSupport::from_stored(refused.stored()), FastModeSupport::Offered);
        assert_eq!(FastModeSupport::from_stored(FastModeSupport::Unknown.stored()), FastModeSupport::Unknown);
    }

    #[test]
    fn fast_mode_change_sends_only_a_different_value() {
        let select = json!({"id": "fast", "type": "select", "currentValue": "off"});
        assert_eq!(fast_mode_change(&select, true), Some(json!("on")));
        assert_eq!(fast_mode_change(&select, false), None);
        let boolean = json!({"id": "fast-mode", "type": "boolean", "currentValue": true});
        assert_eq!(fast_mode_change(&boolean, false), Some(json!(false)));
        assert_eq!(fast_mode_change(&boolean, true), None);
    }
}
