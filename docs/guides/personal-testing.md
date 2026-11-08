# Efficient personal testing

Kai authorized this policy on 2026-11-08. It applies to `codex/personal`, its task
branches and both maintenance automations. It overrides the generic always-full
Playwright, three-reviewer and broad platform-expansion defaults. Upstream CI,
external PR/release procedures and the production safety gates are unchanged.

## Choose once, run once

The coordinator reviews the diff once, records the affected behavior, risk and
exact check commands, then delegates execution to one **`gpt-6.1-terra` subagent**.
Use `fork_turns="none"` and `reasoning_effort="low"` for routine execution. Send
only the worktree, base/head or dirty-diff fingerprint, changed paths, commands,
sandbox/environment and log directory. Do not copy the conversation or repository.
Reuse that worker for a bounded retest. It must not spawn more agents, edit code,
commit, push, deploy, stop production or send production bot messages. The
coordinator owns review, fixes, integration and production acceptance. If Terra
is unavailable, report the blocked checks; do not silently substitute a model or
claim a Terra run. Increase reasoning only for a concrete diagnostic need.

| Situation | Required checks | Do not run by default |
| --- | --- | --- |
| No shipped-code change | Daily current health/cloud and routed read-only MCP; docs/link/diff checks if docs changed | Candidate build, app restart/install, unit/E2E suites |
| Ordinary code change | Affected regression tests; relevant typecheck/fmt/build; sandbox smoke plus changed-flow E2E | Unrelated packages/platforms, whole Playwright, all-target Clippy |
| High risk or unexplained failure | Expanded checks for affected packages and coupled contracts, plus risk-specific cases below | Entire repository solely because one risk keyword matched |
| Weekly regression or demonstrably unbounded impact | Desktop typecheck/unit/whole Playwright; Host fmt/workspace all-target Clippy/full cargo test; affected available phone/platform checks | New VMs/toolchains; production rebuild/reinstall just for tests |

A docs-only change uses artifact/link/diff validation; no app E2E or rebuild.
For ordinary code, one independent reviewer covers correctness, completeness and
cleanup in one pass; the coordinator verifies findings. Add a targeted specialist
only for an unresolved high-risk issue. Do not start three copies of a broad
review automatically. External contributions still follow their security review.

## Select the affected checks

Read the commands for the affected module in [agent build reference](agent-build-commands.md),
not every platform guide. Examples (run from the stated package directory):

- Desktop: `npm run typecheck`, selected `node --test --experimental-strip-types
  <affected.test.ts>`, then `npm run e2e -- e2e/smoke.spec.ts e2e/<changed>.spec.ts`.
- Host: `cargo fmt --all --check`, `cargo test -p codync-host --bin codync-host
  <module_filter>` and a relevant `cargo test -p codync-host --test <suite>`.
  Other workspace crates run only when affected. Clippy is targeted unless weekly
  or affected-package risk requires all targets.
- iOS/Android: affected tests and necessary builds on both when shared phone
  behavior changes; smoke plus changed-area Maestro tags when available. A Host
  change alone does not automatically rebuild both phone apps.
- Cloud/relay: package typecheck and selected tests only when their code or
  contracts change. Do not deploy the cloud as a side effect of personal testing.

Escalation must cover the actual risk before deployment:

- Database/migration: synthetic upgrade and compatible App-only rollback, original
  IDs/content/configuration/permissions preserved; never mutate production data.
- Auth/Keychain/identity: revocation, account/store replacement and credential
  boundaries; never reset credentials or test the production Keychain mutably.
- Protocol/cloud/MCP: released-mobile floors, affected requests/decoders and wire
  contracts; new-client tests alone do not prove released-client compatibility.
- Updates/signing/packaging/lifecycle/dependencies: applicable guards, signed build,
  startup/stop/reconnect and dependent-package tests. Broaden to complete affected
  suites or full regression when the impact cannot be bounded, and say why.

Keep cross-client behavior parity and relevant client checks. Missing UI/audio or
physical phone acceptance is `pending_user_acceptance` under the deployment policy;
known automation, compatibility, data or product failures block promotion.

## Reuse valid evidence

Before running a command, inspect receipts and the validation manifest. Reuse only
successful evidence for the same tested source revision, command, dependency locks,
toolchain, environment/variant and relevant sandbox/test inputs. Record these in a
stable cache key; unknown inputs invalidate reuse. A dirty candidate includes its
diff/content fingerprint. Any relevant edit, failure, new bug report or changed
input invalidates the affected result, not every unrelated check.

Docs-only HEAD changes may refer to the original tested/installed commit after an
explicit shipped-code diff check; never relabel old results as testing a new commit.
Keep `reviewedCommit` separate from `installedCommit`. A weekly run may reuse a
complete matching check executed within the last seven days (for example, an
expanded deployment check), while retaining its original `executedAt`, `testedCommit`,
coverage/skips and log path. Reuse never refreshes evidence age. Run expired or
missing checks; preserve unresolved weekly failures as deployment blockers.

Current production health/cloud/MCP and pending UI evidence are checked separately;
old source-test evidence cannot certify today's runtime state. Backup-restore
validation can reuse a verified archive only when its current checksum and restore
procedure match; every actual deployment still takes each machine's fresh complete
backup. Restore SQLite main/WAL/SHM only in a private copy. Both Macs use the same
accepted signed artifact: test/build once on MacBook, do not rebuild/retest the
same artifact on Mini; Mini still needs its own backup and preservation/MCP checks.

## Small context and output

- Save each command's complete stdout/stderr directly to one local log. Record
  command/cwd, real exit code, elapsed seconds, source/cache key and evidence path
  in one `validation.json` (or the existing daily/weekly receipt), outside Git.
  Do not add another reporting framework or duplicate a receipt in many formats.
- Worker final: at most 8 lines / about 300 Chinese characters: profile, actual
  model, tested revision, pass/fail/skip/pending totals, duration, cache references,
  first actionable failure and absolute log/manifest paths. No successful log dump.
- On failure, return at most the first relevant 20 log lines with line numbers;
  the coordinator reads further only to resolve a specific question. Do not reread
  full logs, complete diffs or historical reports at every poll.
- Wait for completion or meaningful changes; no rapid output polling. Fix once,
  rerun only invalidated checks/failed flows once. A further failure stays failed
  with evidence and a follow-up plan; do not loop until green or weaken tests.
- Keep deterministic lightweight checks in one worker. Host test invocations run
  sequentially; one sandbox worker and one Maestro/emulator lock owner at a time.
  Reuse valid dependency/build caches; no unconditional `npm ci` or tool install.

## Delegation contract

```text
model: gpt-6.1-terra; fork_turns: none; reasoning_effort: low
role: test runner, no edits/commits/deployment/extra agents
worktree: <absolute task checkout>; base/head: <exact refs/fingerprint>
profile: docs | focused | expanded | weekly
scope: <changed behaviors and paths>; commands: <exact commands + cwd>
environment: <unique matching sandbox name/ports and existing toolchains>
artifacts: <private log/validation.json directory>
reuse: <matching original receipt/keys, or none>
return: <=8 lines, status/counts/exit codes/duration/model/revision/log paths
```

The model running the test runner is distinct from the bot/model exercised by a
fixture. This rule neither changes production bot model settings nor automation
coordinator models. If a non-Codex coordinator cannot create this model, it must
hand the compact test contract to an available Codex runner or report unavailable;
it must not invoke the legacy Opus-heavy workflow for personal work.
