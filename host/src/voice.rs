//! Realtime voice for calls on the user's own provider keys (`OpenAI` Realtime,
//! Gemini Live). The keys stay in this computer's vault (`keys`); a client starting a
//! call gets a short-lived credential minted here and then streams audio to the
//! provider directly, so audio never crosses the channel or the relay.

mod keys;
pub(crate) mod live;
mod models;

use crate::store::Store;
use anyhow::{Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use keys::Access;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(15);
/// Speech to text: keep the speaker's language and script, and coding terms as written.
const TRANSCRIBE_HINT: &str = "Keep the speaker's language and script. Keep code, file and product names as written.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "gemini")]
    Gemini,
}

impl Provider {
    const ALL: [Self; 2] = [Self::OpenAi, Self::Gemini];

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "openai" => Ok(Self::OpenAi),
            "gemini" => Ok(Self::Gemini),
            _ => bail!("unknown voice provider `{s}`"),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
        }
    }

    /// The defaults until this computer has seen the provider's own list (`refresh_loop`).
    fn fallback(self) -> Defaults {
        let (realtime, transcribe, speech) = match self {
            Self::OpenAi => ("gpt-realtime-2.1", "gpt-transcribe", "gpt-4o-mini-tts"),
            Self::Gemini => ("gemini-3.8-live", "gemini-3.8-flash", "gemini-3.8-flash-tts"),
        };
        Defaults { realtime: realtime.into(), transcribe: transcribe.into(), speech: speech.into() }
    }
}

/// The model each voice mode uses until the user picks one: the provider's newest, worked out
/// from its model list once a day, so a new release becomes the default without an app update.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Defaults {
    realtime: String,
    transcribe: String,
    speech: String,
}

const REFRESH_EVERY: Duration = Duration::from_hours(24);

fn defaults_key(p: Provider) -> String {
    format!("voice-defaults.{}", p.id())
}

fn defaults(store: &Store, p: Provider) -> Defaults {
    store.kv_get(&defaults_key(p)).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| p.fallback())
}

/// Looks at each keyed provider's model list once a day and keeps its newest models as the defaults.
pub async fn refresh_loop(hub: std::sync::Arc<crate::hub::Hub>) {
    loop {
        for p in Provider::ALL {
            if keys::configured(&hub.store, p).unwrap_or(false)
                && let Err(e) = models(&hub.store, p).await
            {
                tracing::info!(
                    provider = p.id(),
                    error = format!("{e:#}"),
                    "voice model refresh failed; keeping the last defaults"
                );
            }
        }
        tokio::time::sleep(REFRESH_EVERY).await;
    }
}

/// Model ids and voices go into provider requests: letters, digits, `-`, `.`, `_`.
fn checked(s: &str) -> Result<&str> {
    let ok = !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || "-._".contains(c));
    if ok { Ok(s) } else { bail!("`{s}` isn't a model or voice name") }
}

async fn send(p: Provider, req: reqwest::RequestBuilder) -> Result<Value> {
    let res = req.timeout(TIMEOUT).send().await.map_err(|_| anyhow!("Can't reach {}.", p.name()))?;
    let status = res.status();
    let v: Value = serde_json::from_str(&res.text().await.unwrap_or_default()).unwrap_or(Value::Null);
    if !status.is_success() {
        // The provider's own message ("Incorrect API key provided…") says what to fix.
        let message =
            v["error"]["message"].as_str().map_or_else(|| format!("{} answered {status}.", p.name()), str::to_owned);
        bail!(message);
    }
    Ok(v)
}

/// A credential for one call's audio session: an `OpenAI` client secret (10 minutes)
/// or a one-use Gemini ephemeral token.
async fn mint(p: Provider, access: &Access, model: &str, voice: &str) -> Result<String> {
    let v = match p {
        Provider::OpenAi => {
            let body = json!({
                "expires_after": {"anchor": "created_at", "seconds": 600},
                "session": {"type": "realtime", "model": model, "audio": {"output": {"voice": voice}}},
            });
            send(p, access.request(p, Method::POST, "/v1/realtime/client_secrets").json(&body)).await?
        }
        Provider::Gemini => {
            let now = chrono::Utc::now();
            let body = json!({
                "uses": 1,
                "expireTime": (now + chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "newSessionExpireTime": (now + chrono::Duration::minutes(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            });
            send(p, access.request(p, Method::POST, "/v1alpha/auth_tokens").json(&body)).await?
        }
    };
    let field = match p {
        Provider::OpenAi => "value",
        Provider::Gemini => "name",
    };
    v[field]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{} sent no session credential.", p.name()))
}

/// Which providers have a key here, with their default models.
pub fn status(store: &Store) -> Result<Value> {
    status_for(store, |p| keys::configured(store, p))
}

fn status_for(store: &Store, mut configured: impl FnMut(Provider) -> Result<bool>) -> Result<Value> {
    let providers = Provider::ALL
        .iter()
        .map(|&p| {
            let defaults = defaults(store, p);
            Ok(json!({"provider": p.id(), "configured": configured(p)?, "defaults": defaults}))
        })
        .collect::<Result<Vec<Value>>>()?;
    Ok(json!({"supportsLive": !crate::remote::managed::is_managed(store), "providers": providers}))
}

/// Checks an available model before saving the key; Live-only access does not start a billable session.
pub async fn set_key(store: &Store, p: Provider, key: &str) -> Result<Value> {
    let key = key.trim();
    let discovered = if key.is_empty() {
        None
    } else {
        match p {
            Provider::OpenAi if !crate::remote::managed::is_managed(store) => {
                models::validate_openai_key(key, &defaults(store, p).realtime).await?
            }
            Provider::OpenAi => {
                mint(p, &Access::direct(p, key), &defaults(store, p).realtime, "marin").await?;
                None
            }
            Provider::Gemini => {
                mint(p, &Access::direct(p, key), &defaults(store, p).realtime, "Kore").await?;
                None
            }
        }
    };
    keys::save(store, p, key)?;
    if let Some(defaults) = discovered {
        store.kv_set(&defaults_key(p), &serde_json::to_string(&defaults)?)?;
    } else if !key.is_empty() && (p == Provider::Gemini || crate::remote::managed::is_managed(store)) {
        let _ = models(store, p).await;
    }
    status(store)
}

/// A credential for a call starting now.
pub async fn session(store: &Store, p: Provider, model: &str, voice: &str) -> Result<Value> {
    if p == Provider::OpenAi && live::is_model(model) {
        live::validate_access(&keys::direct_key(store, p)?, model).await?;
        return Ok(json!({"validated": true}));
    }
    let access = keys::access(store, p)?;
    Ok(json!({"credential": mint(p, &access, checked(model)?, checked(voice)?).await?}))
}

pub use models::{models, models_for_client};

/// Names a speaker is likely to say that speech-to-text would otherwise misspell: the app and
/// the bots on this computer.
fn spoken_names(store: &Store) -> Vec<String> {
    let mut names = vec!["Codync".to_owned()];
    for bot in store.bots().unwrap_or_default() {
        if !bot.deleted && !names.contains(&bot.config.name) {
            names.push(bot.config.name);
        }
    }
    names.truncate(50);
    names
}

fn names_hint(names: &[String]) -> String {
    format!("Names that may come up: {}.", names.join(", "))
}

/// Speech to text for one utterance (`audio`: base64 WAV, 16 kHz mono).
pub async fn transcribe(store: &Store, p: Provider, model: &str, audio: &str) -> Result<Value> {
    let access = keys::access(store, p)?;
    let model = checked(model)?;
    let wav = STANDARD.decode(audio).map_err(|_| anyhow!("audio must be base64"))?;
    let names = spoken_names(store);
    let text = match p {
        Provider::OpenAi => {
            let boundary = format!("codync-{}", uuid::Uuid::new_v4().simple());
            let mut body = Vec::with_capacity(wav.len() + 512);
            body.extend_from_slice(
                format!("--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\n{model}\r\n").as_bytes(),
            );
            // Keeps Traditional Chinese from coming back Simplified, and code terms intact.
            let mut field = |name: &str, value: &str| {
                body.extend_from_slice(
                    format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                        .as_bytes(),
                );
            };
            field("prompt", &format!("{TRANSCRIBE_HINT} {}", names_hint(&names)));
            // Only gpt-transcribe takes keywords; the older models reject the field.
            if model.starts_with("gpt-transcribe") {
                for name in &names {
                    field("keywords[]", name);
                }
            }
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"speech.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(&wav);
            body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
            let req = access
                .request(p, Method::POST, "/v1/audio/transcriptions")
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(body);
            send(p, req).await?["text"].as_str().unwrap_or_default().to_owned()
        }
        Provider::Gemini => {
            let body = json!({"contents": [{"parts": [
                {"text": format!("Transcribe this audio exactly as spoken. {TRANSCRIBE_HINT} {} Reply with the transcript only; reply with nothing if there is no speech.", names_hint(&names))},
                {"inlineData": {"mimeType": "audio/wav", "data": audio}},
            ]}]});
            let v = send(p, gemini_generate(&access, model).json(&body)).await?;
            v["candidates"][0]["content"]["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|part| part["text"].as_str())
                .collect::<String>()
        }
    };
    Ok(json!({"text": text.trim()}))
}

/// Text to speech for one stretch of a reply: `{audio: base64, format: "aac" | "wav"}`.
pub async fn speak(store: &Store, p: Provider, model: &str, voice: &str, text: &str) -> Result<Value> {
    let access = keys::access(store, p)?;
    let (model, voice) = (checked(model)?, checked(voice)?);
    if text.trim().is_empty() || text.len() > 4000 {
        bail!("text must be 1–4000 bytes");
    }
    match p {
        Provider::OpenAi => {
            let body = json!({"model": model, "voice": voice, "input": text, "response_format": "aac"});
            let res = access
                .request(p, Method::POST, "/v1/audio/speech")
                .json(&body)
                .timeout(TIMEOUT * 4)
                .send()
                .await
                .map_err(|_| anyhow!("Can't reach {}.", p.name()))?;
            let status = res.status();
            let bytes = res.bytes().await.map_err(|_| anyhow!("{} sent no audio.", p.name()))?;
            if !status.is_success() {
                let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                bail!(
                    v["error"]["message"]
                        .as_str()
                        .map_or_else(|| format!("{} answered {status}.", p.name()), str::to_owned)
                );
            }
            Ok(json!({"audio": STANDARD.encode(&bytes), "format": "aac"}))
        }
        Provider::Gemini => {
            let body = json!({
                "contents": [{"parts": [{"text": text}]}],
                "generationConfig": {
                    "responseModalities": ["AUDIO"],
                    "speechConfig": {"voiceConfig": {"prebuiltVoiceConfig": {"voiceName": voice}}},
                },
            });
            let v = send(p, gemini_generate(&access, model).json(&body)).await?;
            let pcm = v["candidates"][0]["content"]["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .find_map(|part| part["inlineData"]["data"].as_str())
                .ok_or_else(|| anyhow!("{} sent no audio.", p.name()))?;
            let pcm = STANDARD.decode(pcm).map_err(|_| anyhow!("{} sent unreadable audio.", p.name()))?;
            Ok(json!({"audio": STANDARD.encode(wav(&pcm, 24_000)), "format": "wav"}))
        }
    }
}

fn gemini_generate(access: &Access, model: &str) -> reqwest::RequestBuilder {
    access
        .request(Provider::Gemini, Method::POST, &format!("/v1beta/models/{model}:generateContent"))
        .timeout(TIMEOUT * 4)
}

/// 16-bit mono PCM wrapped in a WAV header (Gemini speech is raw PCM).
fn wav(pcm: &[u8], rate: u32) -> Vec<u8> {
    let len = u32::try_from(pcm.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(pcm.len() + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_pcm() {
        let out = wav(&[0, 0, 1, 0], 24_000);
        assert_eq!(out.len(), 48);
        assert_eq!(&out[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes([out[40], out[41], out[42], out[43]]), 4);
    }

    #[test]
    fn names_are_checked() {
        assert!(checked("gpt-realtime-2.1").is_ok());
        assert!(checked("Kore").is_ok());
        assert!(checked("x\"}").is_err());
        assert!(checked("").is_err());
        assert!(Provider::parse("openai").is_ok());
        assert!(Provider::parse("anthropic").is_err());
    }
    #[test]
    fn status_advertises_live_only_on_local_hosts_and_keeps_realtime_defaults() {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let local = status_for(&store, |p| Ok(p == Provider::OpenAi)).unwrap();
        assert_eq!(local["supportsLive"], true);
        assert_eq!(local["providers"][0]["configured"], true);
        assert_eq!(local["providers"][0]["defaults"]["realtime"], "gpt-realtime-2.1");
        store.kv_set("cloud_managed", "1").unwrap();
        let cloud = status_for(&store, |_| Ok(true)).unwrap();
        assert_eq!(cloud["supportsLive"], false);
        assert!(keys::direct_key(&store, Provider::OpenAi).unwrap_err().to_string().contains("cloud"));
    }

}
