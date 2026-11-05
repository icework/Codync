# Personal Live diagnostics

Local host logs use the target `codync_host::voice_diagnostics`. They are observational and do not
change permissions, pairing, mobile compatibility floors or production data.
Read the actual service's CODYNC_HOME first; the standard main location is
`~/.codync/host.log`. A sandbox has its own `~/.codync-dev-<name>/host.log`.

```sh
# Audio client MacBook: local engine quality and view lifecycle.
rg 'codync_host::voice_diagnostics' ~/.codync/host.log
# Task host Mini: setup, tool handoff, cue emission and Live RPC outcomes.
ssh macmini 'rg codync_host::voice_diagnostics ~/.codync/host.log'
```

Correlate an individual call using its UUID (`call` in local logs). Non-UUID IDs
are logged as `unknown`. Call IDs never enter PostHog properties. Do not copy full
host logs into public reports; other existing log categories may contain private data.

| Observation | Meaning / limit |
| --- | --- |
| `start_live`, `setup` | Host setup request duration and success; not audio readiness |
| `first_input_transcript` / `first_output_transcript` | First transcript received in this provider session; not audible playback or exact first-word latency |
| `live_event` | Allowlisted Responses lifecycle/completed-item event, Host RPC duration and result; no raw provider event/message |
| `tool_completed` | Allowlisted tool, execution duration and success; send_to_bot means queued, not task complete |
| `cue_ready` | Bot reply/notice/progress available to the call, bytes, broadcast subscriber count and elapsed time since call start; zero subscribers means no live delivery, and a positive count is not confirmed client receipt |
| `live_update` | Time to prepare provider append events, input byte count and output event count; not playback acknowledgement |
| `end_call`, `client_requested` | Host received client finalization and optional desktop endReason (user/view_unmounted), even if no engine could start; older clients omit it |
| `client_quality`, `engine_ended` | MacBook engine quality and optional user/view_unmounted/engine_replaced reason; replacement is not a whole-call hangup |
| `client_window`, `recovery` | Existing desktop window recovery telemetry reached the local host |

Client quality keeps the existing ready_ms/reconnects/stuck/failure analytics and
adds enum-only end_reason and failure_detail. Live distinguishes session_closed,
provider_error, host_rpc and queue_full; it never logs the free-form failure string.
`view_unmounted` describes the observed React lifecycle, not why the view went away.
A crash or lost local Host connection can prevent delivery; missing logs are not proof
that an event did not happen. Existing host log lifecycle/retention applies. Transcript
fragments are not logged individually (only the first input/output timestamp), so
normal speech does not produce a log line per fragment.

## Evaluation sequence

Use synthetic data in an isolated worktree/sandbox, never mutate production chats.
Compare the same short question, a current-fact lookup, an ambiguous name plus a
correction, and a requested detailed research task. Check time to useful answer,
number of send_to_bot actions, time from task dispatch to reply cue, output size,
provider errors, and whether the user can interrupt/continue speaking. Test manual
hangup, leaving the desktop conversation, provider closure and host disconnection.
A scripted provider can check ordering and lifecycle, but cannot validate real model
routing, audio quality or perceived latency. Record those as separate real-audio checks.

Prompt and event choices follow OpenAI's [Live prompting](https://developers.openai.com/api/docs/guides/live-prompting)
and [delegation guidance](https://developers.openai.com/api/docs/guides/live-delegation).
