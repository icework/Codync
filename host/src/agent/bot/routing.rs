//! Applying a conversation's route, without changing the bot's saved defaults.
use super::{Actor, launch::select_model};
use crate::agent::routing::{self, Route};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::time::Duration;

impl Actor {
    pub(super) fn announce_route(&self, sid: &str, force: bool) -> Result<()> {
        if (self.session_fresh || force)
            && let Some(mut route) = routing::saved(&self.hub.store, &self.cfg, sid)?
        {
            route.fast = self
                .conn
                .as_ref()
                .and_then(|c| c.fast_options.get(sid))
                .is_some_and(crate::agent::config::fast_mode_on);
            self.notice(&route.summary(), super::NoticeStyle::Info);
        }
        Ok(())
    }

    pub(super) async fn select_routed_model(&mut self, res: &Value, sid: &str, route: Option<&Route>) -> Result<Value> {
        let cfg = route.map_or_else(|| self.cfg.clone(), |r| r.config(&self.cfg));
        let conn = self.conn.as_ref().ok_or_else(|| anyhow!("agent not running"))?;
        if route.is_none() {
            return select_model(&conn.acp, res, sid, &cfg).await;
        }
        if !res["configOptions"].is_array() {
            if let Some(conn) = self.conn.take() {
                conn.acp.kill().await;
            }
            anyhow::bail!("The agent did not provide routing configuration; no prompt was sent");
        }
        let selected = tokio::time::timeout(Duration::from_secs(20), select_model(&conn.acp, res, sid, &cfg))
            .await
            .context("Model selection timed out")
            .and_then(|r| r)
            .and_then(|v| {
                if let Some(r) = route {
                    routing::verify(&v, r)?;
                }
                Ok(v)
            });
        if selected.is_err()
            && let Some(conn) = self.conn.take()
        {
            // Do not leave an unacknowledged or partially configured session available for a prompt.
            conn.acp.kill().await;
        }
        selected
    }

    pub(super) fn routed_fast(&self, sid: &str) -> bool {
        match routing::saved(&self.hub.store, &self.cfg, sid) {
            Ok(Some(route)) => route.fast && routing::allows_fast(&self.hub, &self.cfg),
            Ok(None) => !routing::enabled(&self.cfg) && self.cfg.fast_mode,
            Err(e) => {
                tracing::warn!(bot = %self.cfg.id, error = %e, "couldn't read the conversation route");
                false
            }
        }
    }
}
