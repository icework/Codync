//! Local, content-free voice timing. No transcripts, tool arguments, provider messages or credentials.

use serde_json::Value;
use std::time::Instant;

pub struct Observation {
    started: Instant,
    call: Option<String>,
    operation: &'static str,
    stage: &'static str,
    bytes: usize,
    reason: &'static str,
    failure: &'static str,
    failure_kind: &'static str,
    ready_ms: Option<u64>,
    reconnects: Option<u64>,
}

impl Observation {
    pub fn start(method: &str, body: &Value) -> Option<Self> {
        let (operation, stage) = match method {
            "startLiveCall" => ("start_live", "setup"),
            "startCall" => ("start_call", "setup"),
            "callTool" => ("tool", tool_name(body["name"].as_str().unwrap_or_default())),
            "callLiveUpdate" => ("live_update", one_of(body["kind"].as_str(), &["reply", "notice", "progress"])),
            "endCall" => ("end_call", "client_requested"),
            "callLiveEvent" => {
                let e = &body["event"];
                let event = if e["type"] == "response.event" { &e["event"]["type"] } else { &e["type"] };
                let stage = one_of(
                    event.as_str(),
                    &[
                        "response.created",
                        "response.output_item.done",
                        "response.completed",
                        "response.failed",
                        "response.incomplete",
                        "response.cancelled",
                        "session.closed",
                        "error",
                        "session.error",
                    ],
                );
                if stage == "other" {
                    return None;
                }
                ("live_event", stage)
            }
            "track" if body["event"] == "voice_call_quality" => ("client_quality", "engine_ended"),
            "track" if body["event"] == "window_recovery" => ("client_window", "recovery"),
            _ => return None,
        };
        let call = body["callId"]
            .as_str()
            .or_else(|| body["call_id"].as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .map(|id| id.to_string());
        Some(Self {
            started: Instant::now(),
            call,
            operation,
            stage,
            bytes: body["text"].as_str().map_or(0, str::len),
            ready_ms: body["ready_ms"].as_u64(),
            reconnects: body["reconnects"].as_u64(),
            failure_kind: one_of(body["failure"].as_str(), &["none", "microphone", "connection", "provider"]),
            reason: one_of(
                body["endReason"].as_str().or_else(|| body["end_reason"].as_str()),
                &["user", "view_unmounted", "engine_replaced"],
            ),
            failure: one_of(
                body["failure_detail"].as_str(),
                &["none", "session_closed", "provider_error", "host_rpc", "queue_full", "unspecified"],
            ),
        })
    }

    pub fn finish(self, result: &anyhow::Result<Value>) {
        let events = result.as_ref().ok().and_then(|v| v["events"].as_array()).map_or(0, Vec::len);
        tracing::info!(target: "codync_host::voice_diagnostics", call = self.call.as_deref().unwrap_or("unknown"),
            operation = self.operation, stage = self.stage, elapsed_ms = self.started.elapsed().as_millis(),
            ok = result.is_ok(), input_bytes = self.bytes, output_events = events,
            end_reason = self.reason, failure_detail = self.failure, failure = self.failure_kind,
            ready_ms = self.ready_ms, reconnects = self.reconnects, "voice diagnostic");
    }
}

pub(super) fn call_id(value: &str) -> String {
    uuid::Uuid::parse_str(value).map_or_else(|_| "unknown".into(), |id| id.to_string())
}

fn one_of(value: Option<&str>, allowed: &[&'static str]) -> &'static str {
    allowed.iter().copied().find(|s| Some(*s) == value).unwrap_or("other")
}

pub(super) fn tool_name(name: &str) -> &'static str {
    one_of(
        Some(name),
        &["send_to_bot", "bot_status", "recent_messages", "search_memory", "answer_approval", "stop_bot"],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn logs_only_allowlisted_metadata_and_uuid_identifiers() {
        let body = json!({"callId":"private text", "kind":"secret", "text":"private contents"});
        let trace = Observation::start("callLiveUpdate", &body).unwrap();
        assert!(trace.call.is_none());
        assert_eq!(trace.stage, "other");
        assert_eq!(trace.bytes, 16);
        assert_eq!(tool_name("secret tool argument"), "other");
        assert_eq!(Observation::start("endCall", &json!({"endReason":"user"})).unwrap().reason, "user");
        assert_eq!(Observation::start("endCall", &json!({})).unwrap().reason, "other");
        let trace = Observation::start(
            "track",
            &json!({"event":"voice_call_quality",
            "call_id":"2392be1f-9ef1-43fd-aaa5-a2e1eae16bad", "end_reason":"view_unmounted",
            "failure_detail":"host_rpc", "error":"secret provider response"}),
        )
        .unwrap();
        assert!(trace.call.is_some());
        assert_eq!((trace.reason, trace.failure), ("view_unmounted", "host_rpc"));
    }

    #[test]
    fn transcript_fragments_and_unrelated_requests_do_not_flood_logs() {
        assert!(Observation::start("send", &json!({"text":"private"})).is_none());
        assert!(
            Observation::start(
                "callLiveEvent",
                &json!({"event":{"type":"session.input_transcript.delta","delta":"private"}})
            )
            .is_none()
        );
        let trace = Observation::start(
            "callLiveEvent",
            &json!({"event":{"type":"response.event",
            "event":{"type":"response.failed","response":{"error":{"message":"private"}}}}}),
        )
        .unwrap();
        assert_eq!(trace.stage, "response.failed");
    }
}
