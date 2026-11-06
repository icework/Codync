# Codex smart routing

A Codex bot's **Smart routing** switch enables OpenAI Decisions for each new conversation. The Rust host chooses and applies the model and reasoning effort before sending a prompt. Only desktop settings expose the opt-in switch in this first version. Phones and the terminal keep their existing settings UI; their ordinary edits preserve the host field, and chat notices still show routing outcomes. Other agents and fallback-chain steps retain their manual settings.

## Credentials and scope

Decisions uses `POST https://api.openai.com/v1/decisions`, model `gpt-6-luna`, with a typed `choice` question. It requires a separately billed OpenAI platform key; a Codex ChatGPT login is not that key. The host reuses the OpenAI key already stored in its encrypted call-settings vault. No new environment variable or second credential store is introduced. Clients never receive the key. No experimental Codex permission/guardian flag is enabled.

The classifier receives at most 16,000 characters of the initiating request and 2,000 characters of the bot's standing instructions. It does not receive tool results, project files, memory or full chat history. These strings are classification data; policy and allowed choices are separate. No prompt, key, raw response or free-form rationale is persisted or included in analytics.

An eight-second deadline covers key lookup and the request. Redirects are disabled, the response is bounded to 64 KiB, and the answer must name exactly one offered choice. Missing credentials, network/API failures, timeout or malformed responses produce a visible notice and use the bot's saved model/effort at standard speed. This fallback is recorded as unavailable, not presented as a Decisions selection. Unsupported or unacknowledged model settings stop setup without sending the task.

## Initial policy

The profiles in `host/src/agent/routing/profiles.rs` encode the personal benchmark completed on November 6, 2026. This is a small, tool-dependent experiment, not a general model ranking. Only profiles whose model appears in the current Codex session catalog are offered.

| Work | Preferred profile | Evidence used |
| --- | --- | --- |
| Short conversation, rewriting, translation, bounded offline work | Terra low | About 8.5 seconds in the initial simple-task sample |
| Simple web lookup, ordinary multi-source research with many calls but little hard reasoning | Terra high | Simple web Fast: 11.0 seconds, 12/12 passes; medium Fast did not improve this |
| Exhaustive research, exact counts/dates/units and careful source comparison | Sol low | Expanded research: 18/18 passes, stronger completeness, about 58.7 seconds |
| Difficult multi-step reasoning, implementation, architecture and debugging | Sol high | Initial complex-task pass rate 18/18, versus Sol low 15/18 |

The classifier selects one complete profile with standard or Fast speed. It cannot invent arbitrary model/effort strings. A model switch is followed by setting effort; both must be acknowledged by the ACP adapter. Approval mode and permissions stay with the bot.

## Fast and quota

Fast is offered only when both primary and secondary Codex plan windows are present and every reported window has at least 50% remaining, each reset is in the future, and the reading is no more than 10 minutes old. This conservative initial threshold is a product policy, not a benchmark finding. Unknown, empty, expired, nonfinite or note-only readings do not qualify. Custom commands and saved Codex environments may refer to a different account, so they use standard speed. Legitimate single-window plans also conservatively use standard speed in this version. There is no new polling loop: the existing usage reader provides the snapshot.

The policy prefers Fast when eligible, unless the request prioritizes conserving usage or describes unattended work. The host checks quota again after the decision and before each subsequent turn. If standard speed is required but cannot be acknowledged, the prompt is withheld. An agent may refuse Fast and run at standard speed; the chat notice describes the acknowledged setting.

## Conversation lifetime

The route is stored per bot and ACP session under `routing.<bot>.<session>`, separate from `BotConfig.model`, `modelOptions` and `fastMode`. Follow-up messages, host restarts, session reloads and process recovery reuse the saved route. A failed reload that replaces an existing session carries its route forward rather than classifying the recovery again. Routine sessions have their own routes. An old un-routed session recovered after a config-save crash is classified once before it can run; its prior conversation is preserved.

Turning the switch on or off starts a fresh session once any active turn finishes, like changing the manual model. In the main chat, more than 30 minutes since the prior turn ended triggers the existing handoff flow and a new routed conversation, regardless of context size. Ordinary idle process reclamation does not itself discard the route. Deleting a bot removes its route records along with other session metadata.

## Validation and telemetry

`session_routed` records classifier outcome, one allowlisted profile, requested Fast, and duration; `bot_updated.fields` records use of the switch. Adapter acknowledgement failures remain failed turns, not successful routing execution. The event is classification outcome, not a claim that the task succeeded.

Unit tests cover offered choices, response validation, bounded prompt data, quota uncertainty and session persistence. `routing_e2e` runs the real host/API against a deterministic ACP fixture to verify safe credential fallback, route reuse after restart, untouched manual defaults and no prompt on missing acknowledgement. This fixture does not establish live Decisions access or live-model quality. Live acceptance needs an eligible OpenAI platform credential and a separately signed-in Codex installation.

References: [Decisions guide](https://developers.openai.com/api/docs/guides/decisions), [create schema](https://developers.openai.com/api/reference/typescript/resources/decisions/methods/create).
