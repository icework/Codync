//! Fast uses more subscription allowance. Unknown or mismatched accounts never qualify.
use crate::{LockExt, hub::Hub, store::BotConfig, usage::Provider};

const MAX_AGE_MS: i64 = 10 * 60 * 1000;
const MAX_USED_PERCENT: f64 = 50.0;

pub fn allows_fast(hub: &Hub, cfg: &BotConfig) -> bool {
    // The usage reader probes the default CLI account. Custom commands or saved login
    // environments may select another account, whose quota this reading cannot establish.
    if cfg.backend != "codex"
        || cfg.command.as_deref().is_some_and(|s| !s.trim().is_empty())
        || hub.store.kv_read("agent-env:codex").map_or(true, |v| v.is_some())
    {
        return false;
    }
    let usage = hub.usage.locked();
    ample(usage.providers.iter().find(|p| p.id == "codex"), crate::store::now_ms())
}

pub(super) fn ample(provider: Option<&Provider>, now: i64) -> bool {
    let Some(p) = provider else { return false };
    if !(0..=MAX_AGE_MS).contains(&(now - p.updated_at)) {
        return false;
    }
    // A damaged/expired secondary window is omitted by the usage reader. Until it
    // can distinguish legitimate single-window plans, those conservatively stay standard.
    if !["primary", "secondary"].iter().all(|id| p.windows.iter().any(|w| w.id == *id)) {
        return false;
    }
    p.windows.iter().all(|w| {
        w.text.is_none()
            && w.percent.is_finite()
            && (0.0..=MAX_USED_PERCENT).contains(&w.percent)
            && w.resets_at.is_some_and(|reset| reset > now)
    })
}
