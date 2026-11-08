# Personal production builds

Kai's `codex/personal` tracks upstream `origin/dev` and is backed up to
`fork/codex/personal` (`icework/Codync`). Never push personal changes to `origin`.
The persistent checkout is `/Users/kaiwu/.codex/worktrees/codync-personal/Codync`.
Keep this worktree; the shared checkout stays on upstream `dev`.

## Personal feature workflow

These rules govern Kai's personal branch and all task branches derived from it.
They override upstream instructions to start from/merge into `dev`, rebase with
an autostash, push to `origin`, or remove older mobile contracts by raising a floor.

1. Fetch `origin` and `fork`, inspect status and refs, and preserve others' work.
   The base is current accepted `codex/personal`, not `origin/dev`, `main` or an
   old feature branch. If the fork is ahead, reconcile it safely first; stop on
   divergence. Fetch does not authorize deploying incoming upstream changes.
2. Every feature (including small ones), fix and documentation task uses a separate
   `codex/<task>` branch and worktree based on that personal commit. Use the managed
   worktree tool when available. Show `pwd`, branch and base commit before edits.
   The permanent personal checkout is for integration/maintenance, not feature work.
3. Implement, build and test in that worktree, with a unique sandbox and matching
   `--name` / `CODYNC_SANDBOX_NAME`; never use production data in mutable tests.
   Apply the normal review/after-change requirements to source changes. Documentation
   changes need diff/link checks, not a production rebuild or app restart.
4. Before integration, inspect the complete change against latest personal. If it
   moved, merge personal into the task branch, review conflicts and rerun affected
   checks. Do not rebase published personal/task history. Conflicts follow the
   stop-and-offer-options policy in the daily pipeline.
5. Under the shared update lock, recheck the personal tip and clean status, then
   fast-forward personal to the accepted task (or use a reviewed merge if needed).
   Push only `fork codex/personal`; optional task backups also go only to `fork`.
   Do not push `origin`, force-push, tag a public release or deploy the cloud as a
   side effect. Archive only this task's worktree after successful integration.
6. Source integration is not production deployment. Deploy using the MacBook-first,
   same-artifact-to-Mini pipeline below. Documentation-only commits change the
   reviewed source, never the recorded installed source or installed artifact hash.

## Authorized deployment, then user acceptance

On 2026-11-05 Kai explicitly authorized merging and deploying new personal apps,
then testing them himself and rolling back if needed. This overrides earlier
requirements to wait for manual real-audio, phone or production UI acceptance.
Do not ask again for routine deployment approval or block a working candidate
solely because a physical phone/audio test or UI tool is unavailable.

Required pre-deployment checks remain: review, applicable automated tests/build,
known protocol compatibility, idle state (including active calls), independent
backups, signature/identity and personal updater guards. Known failures, unresolved
merge conflicts, data-integrity errors or a proven incompatible production mobile
contract still stop rollout. Use the validation tier appropriate to the actual
risk; missing manual acceptance alone does not require repeating passed suites.

Deploy MacBook first; verify startup, installed hashes, preserved identity/data,
cloud and routed MCP before updating Mini with the same signed artifact. Record
`userAcceptance: pending`, and unperformed audio/mobile/UI checks as
`pending_user_acceptance`, separately from successful installation/backend checks.
Do not claim these flows passed. No new user messages or mutable tests against
production bots are authorized by this acceptance policy.

If Kai reports a regression, or post-install checks detect one, stop further
rollout and safely restore the previous app. Preserve post-update conversations
and account state; roll back only the app when the database is backward compatible.
For migrations, preserve a failed-state backup and verify a data-preserving recovery
plan before reverting data; never overwrite new conversations with an old snapshot.
Keep a verified rollback archive on each machine until the next accepted version.

## Production mobile compatibility gate

Kai may keep using the upstream **production iOS app**, and upstream production
Android when applicable, while the MacBook and Mini run personal code. This is a
supported combination, not a requirement to install a personal mobile build.
The Mac personal app uses the **main** cloud/account environment; an isolated
Codync Dev sandbox is a different environment and is not a production phone host.

- The baseline follows upstream production iOS; Kai will keep the phone app updated
  (confirmed 2026-11-05). Use the upstream repository's iOS source at the release
  tag/commit corresponding to the latest available production App Store version,
  not a permanently pinned installed phone version or unreleased `origin/dev` HEAD.
  Record that version, source commit and dated release evidence for each affected
  validation. Routine maintenance does not require asking Kai for the phone version.
- A repository tag or newer `origin/main` commit alone does not prove the iOS app is
  available in the store. If a candidate requires an unreleased mobile change, keep
  the currently released contract or defer deployment until that mobile release is
  available and compatibility passes. Check upstream iOS changes proactively, but
  do not treat tests against unreleased source as production-client acceptance.
  Missing manual phone/audio checks are `pending_user_acceptance` under the
  deployment policy above; automated failures or known incompatibility still stop
  deployment and require a concrete recovery or adaptation plan.
- Preserve the baseline's pairing certificates, encrypted handshake, cloud routing,
  RPC names/request defaults, response types, enum handling, SSE catch-up and event
  decoding. Add fields/methods compatibly; keep old request forms and behavior where
  required. A new enum value is only safe if that released client's decoder accepts
  it. Account identity alone does not establish protocol compatibility.
- Check both floors: host `minApp` must admit the baseline mobile version, and the
  personal host version must meet that client's `minHost`. Never falsify versions
  or lower `MIN_APP` without restoring and testing the corresponding contracts.
  Unlike official releases, local personal installation bypasses the upstream
  App Store release gate: run this gate explicitly on every affected deployment.
- New capabilities must be opt-in or negotiated, with existing behavior retained
  for old clients. For example, a released phone without GPT Live UI/audio support
  must retain its existing chat and voice paths; installing GPT Live on the Mac
  cannot add phone-side controls or WebRTC/audio handling to that phone app.
- For every candidate review the full deployed-to-candidate diff for mobile impact,
  including incoming upstream code. When protocol/auth/cloud/voice behavior or a
  compatibility floor changes, use expanded validation before deployment: run the
  relevant host contract/wire-shape tests and inspect the released baseline client.
  Test that client against an isolated candidate host when the test environment is
  available; otherwise record manual coverage as pending_user_acceptance. New-client tests alone
  do not prove compatibility with the released app; wire snapshots do not cover
  all requests or response variants. Inspect/test affected released decoders too.
- Acceptance covers connection/reconnection, bot list, history/catch-up, sending and
  receiving, approval responses and the existing voice path when affected. Mutating
  tests use disposable test bots/data; do not send production bot messages or alter
  visibility merely to validate. After deployment, check existing phone pairing and
  read-only connection/history on MacBook, then Mini. Preserve both Macs' own data.
- Record client version/build, host commit/hash, routes and checks, evidence/time,
  and `passed` / `failed` / `pending` in the acceptance report/receipt. Affected
  automated compatibility checks that fail or known contract incompatibilities block
  deployment. Missing manual checks follow the authorized user-acceptance policy
  above and do not block deployment by themselves. Unchanged contracts may
  reuse applicable prior evidence with an explicit diff review; routine no-update
  daily checks do not rebuild/reinstall or require a fresh phone test every day.

As of source `c18c2a3039baea9a5ac4199f851147e92127ae03` (2026-11-05),
`host/src/compat.rs` declares `MIN_APP = 2.36.0`. This is a code-level floor,
not evidence that Kai's installed iOS build passed a real-device acceptance test.
Production-mobile E2E remains unverified at this policy's introduction; retain
that gap until concrete evidence resolves it. Kai's installed phone version is
not a separate pinned baseline or a routine maintenance blocker.

## Identity and data

Both Kai's MacBook and `ssh macmini` use `/Applications/Codync.app`, bundle
`com.pokai.Codync`, the **main** account/cloud environment, host port 19222 and
launch-agent `com.pokai.codync.host`. Each computer retains its own `~/.codync`,
`~/Library/Application Support/Codync`, Keychain items, account.bin and pairing
keys. Never copy any of these between computers or from/to Dev. Inspect the
actual service environment before each update, including custom CODYNC_HOME.
Stop on any mismatch instead of initializing a new identity.

The personal app retains the Codync name and adds a mint **P** icon badge.
Only `tools/personal-builder.cjs` selects that icon. Other platforms and Dev
retain their upstream packaging and behavior. No application UI is forked.
The copied upstream icon mark must be refreshed if the upstream mark changes.

## Build and validation

```sh
cd apps/desktop
export PATH="$HOME/.cargo/bin:$PATH"
npm ci
npm run typecheck
# Select focused tests or the full suite using the validation tiers below.
CODYNC_SIGNING_IDENTITY='<valid local Apple signing identity>' node tools/personal-build.mjs
```

The builder uses main identity/cloud for both app and host, native architecture
(the two current Macs are arm64), the pinned Rust toolchain, a bundled Screen
helper and on-device speech helper. Apple intermediate output stays in `build/dd`.
It uses the fast optimized dev-app Rust profile, not a Dev environment. The
signing identity stays outside Git. It does not publish a GitHub release.

The generated account-config has `updateChannel: personal`; the host is compiled
with `CODYNC_UPDATE_CHANNEL=personal`. Both refuse official release installation,
including manual/remote requests, so an upstream update cannot overwrite personal
changes. Official builds keep the existing update path. Preserve production
login/Keychain state; if macOS requires approval after signing identity changes,
ask the user to handle that prompt, never reset accounts or Keychain items.

Use the validation tiers below for upstream maintenance. Follow after-change for
our own source changes. Run selected desktop Playwright tests in an isolated Dev
sandbox (the full suite for weekly/expanded validation); the sandbox name must be
explicit and identical:

```sh
CODYNC_CLERK_PUBLISHABLE_KEY='' CODYNC_CLOUD_URL='' CODYNC_HOST_BIN='<dev host>' npm run dev -- --name personal-check --cdp
CODYNC_SANDBOX_NAME=personal-check npm run e2e
```

This uses the existing local-only mode without borrowing real login credentials.
Cloud account tests explicitly skip with cloud off; report that skip, and verify
real account restoration and MCP separately on each production app. Use the
normal authenticated sandbox with `--cloud` when testing cloud/account behavior;
that requires the development test-account configuration. Do not change tests to
hide failures. Do not package/build into the same output while E2E is running.
Stop only this run's sandbox before packaging; never prune other sandboxes.

## Validation tiers

Kai requested daily lightweight checks and weekly full regression on 2026-11-04.
This policy applies to personal deployment maintenance, not upstream release CI.

### Daily: focused validation before deployment

- Fetch and compare refs/receipts first. With no new shipped code and matching
  installed hashes, do not create a build worktree, rebuild, reinstall or restart.
  Check both hosts' health/cloud status and read-only MCP connectivity, then exit.
  Documentation-only merges can advance the branch without an app rebuild; record
  `reviewedCommit`/`reviewedUpstreamCommit` separately from the actual installed
  source commit. Compare relevant source, dependency and build-config diffs.
- For ordinary updates, review the entire incoming diff, select tests for affected
  modules, typecheck changed desktop code, and run a small isolated app smoke test
  (launch, bot list/history, and the changed flow). Run focused Rust tests and fmt
  when host code changes. Reuse valid dependency caches; run npm ci when needed
  by lockfile/toolchain/cache changes, not unconditionally. Do not run full unit
  suites or all-target clippy on every ordinary update.
- Preserve the deployment gates below: idle checks, local backups, signing and
  updater guards, each machine's identity/account/data integrity, and host-routed
  MCP calls. Accept the MacBook before deploying the exact same artifact to Mini.
- Routine daily production acceptance uses installed hashes, process/health,
  cloud owner/registration, original data and MCP evidence. Full production UI
  walkthroughs belong to weekly validation. Record `validationTier: daily` and
  `accountUI: deferred_to_weekly`; never label an unobserved UI as passed.
  A previous UI-tool-only timeout is retained as pending for the weekly check,
  not a reason to reinstall. Actual login failures or product failures block
  rollout. Only retry an unavailable UI tool once (roughly 60 seconds total).
- Expand to the full checks **before deployment**, even on a weekday, for changes
  to database schemas/migrations, account/authentication/Keychain/identity,
  cloud/MCP transport or compatibility, updater guards, signing/packaging,
  dependency/toolchain security, broad desktop-main/process lifecycle behavior,
  or when the focused checks fail or leave a concrete unresolved concern. Also
  inspect affected real production UI when available; manual UI/audio/mobile checks
  may follow deployment under Kai's explicit user-acceptance policy above. A tool
  outage is pending_user_acceptance, never a passed check. Automated failures and
  known product/data regressions still block rollout.

### Weekly: full regression, without routine production reinstall

Automation `codync` runs Sundays at 05:00 America/Los_Angeles using the same
single-run lock as daily (`codync-personal`).
If the daily run still owns the lock, defer; never overlap or steal a live lock.
Test the currently deployed personal source in an isolated worktree/sandbox;
record exact tested commit, toolchain and artifact hashes. If machines differ,
report that discrepancy and leave deployment recovery to the daily flow.

Run desktop typecheck and the entire unit/Playwright suites, host fmt, all-target
clippy and full cargo tests. Walk through both real production apps read-only
(original account, bot list, existing history, settings), verify both routed MCP
calls, database integrity, and backup archive readability. Test restore using a
private temporary database copy, never over live production data. Build/test the
current source as needed in the sandbox; do not stop, rebuild or reinstall the
production apps just to run regression. Other platforms use existing test
environments only when affected/applicable; report unavailable coverage honestly.

Write `weekly-verification.json` in the private support directory, with tested
commit, checks, timestamps, pending UI checks and status `passed`, `failed`, or
`pending`. A real regression blocks subsequent deployment until resolved; a tool
outage is not a product failure and does not erase earlier pending checks. Never
reuse weekly results for a different source revision. Keep reports concise and
record individual step durations to measure whether the daily path is faster.

## Daily pipeline: MacBook first, then Mac mini

Codex automation `codync-personal` runs at 04:00 America/Los_Angeles. It has
standing authorization to fetch, merge conflict-free upstream changes, test,
push the personal branch to the fork, and sequentially update these two Macs.
The local computer and Codex app must be running. No update may interrupt an
active bot turn. Git success, MacBook deployment and Mac mini deployment are
separate states, each recorded in a local receipt and timestamped Chinese report.

1. Acquire a single-run lock under `~/Library/Application Support/Codync Personal`.
   Record the owning PID and release only your own lock. Inspect a stale lock
   before recovery. Read this guide and current repository instructions. Inspect
   dirty worktrees, running tasks, upstream/fork refs and both installed receipts.
   Preserve other work; do not autostash/reset personal or rewrite its history.
2. Fetch origin/fork. Update personal from its fork counterpart only by safe
   fast-forward; divergent personal history is a blocker. Create an isolated
   candidate worktree/branch, then **merge** `origin/dev`. Never force-push.
3. For conflicts, preserve the current apps and personal branch. Keep the candidate
   and write a report identifying files, both sides' intent, affected behavior and
   concrete options: retain/adapt personal changes; adopt upstream and retire the
   override; or defer. Recommend per conflict and wait for Kai's decision. Do not
   use blanket ours/theirs or silently discard either side. Reuse an existing
   unresolved candidate/report instead of creating one every day.
4. Apply the production mobile compatibility gate above. Review incoming migrations,
   main cloud availability, phone/host compatibility,
   personal updater guards and icon. Select the validation tier above and run its
   applicable tests and sandbox E2E. Check the latest weekly report. A failed
   check blocks rollout. A skip for an unrelated platform is recorded explicitly;
   real production account and MCP checks are mandatory. No upstream change and
   matching daily acceptance receipts on BOTH machines means no rebuild; pending
   weekly-only UI checks remain recorded separately. If only
   Mac mini is behind, retry its deployment of the already accepted artifact.
5. Record each machine's environment, app/version, host path/hash, computer ID
   hash, identity/token hashes, cloud owner hash and MCP settings. Save counts
   and stable IDs/configurations of bots, all existing chat entry IDs/content,
   paired devices, and account.bin hash. Never log credentials or conversation
   content. Read /health busy; defer if true. Check available space and SSH.
6. Before stopping the old app, inspect `com.pokai.Codync.ShipIt` and its
   `~/Library/Caches/com.pokai.Codync.ShipIt/ShipItState.plist` request. A live
   official installer or genuinely pending update blocks replacement: let it
   finish and re-audit, never kill an installer mid-swap. A proven stale request
   (source bundle absent and no running job) may be moved into the private backup
   and its idle launchd job removed. Recheck after app exit and immediately before
   copying the new bundle; an old app can schedule its staged update on Quit.
   On the MacBook, quit the old app and stop only its production host before
   rebuilding. Preserve the production Screen service settings. Back up the whole
   app as a non-launchable archive, host data, desktop settings and launch-agent
   plist locally. Exclude sockets; preserve symlinks and permissions. Make a
   consistent SQLite snapshot and verify integrity/archive readability. A tar restore
   must include `codync.db` and any `codync.db-wal` / `codync.db-shm` under their
   original names in a private directory: stopping the host does not guarantee an
   empty WAL. Compare restored row counts and preserved IDs/content with the
   snapshot; never test recovery by overwriting or checkpointing production data.
   Keep backup directories 0700 and archives 0600. No private data goes to Git.
7. Build the candidate with the local signing identity. On failure restore/start
   the previous MacBook app/service. Verify signature, main environment, P icon,
   bundled speech/Screen helpers and **both** personal updater guards before
   replacement. Confirm personal hasn't moved; fast-forward and push only
   `fork codex/personal`. Any failure after stopping the old service but before replacement, including
   push failure, must restart the previous app/host and verify recovery before
   reporting. Keep the candidate for retry.
8. Replace the whole MacBook bundle; don't merge directories. Explicitly run the
   new `/Applications/Codync.app/Contents/Resources/codync-host install --port 19222`.
   Launch the app; check original account, original bots/history,
   health environment/path/hash and updater exclusion. Compare saved data/identity
   evidence. Perform production UI checks when required by the validation tier.
   Verify MCP list_computers and a host-routed list_bots/read_replies.
   Preserve bot-level assistant visibility; never broaden it to make a test pass.
   The user authorized MCP connectivity on both computers; the initial MacBook
   Cloud assistants switch was off and can be enabled for its existing account.
9. **Only after MacBook acceptance**, package that exact signed app, checksum it,
   transfer the artifact (never data) to Mac mini, and verify checksum/architecture.
   Validate the remote process's explicit exit code and a nonempty structured result;
   an SSH shell exit status alone is insufficient and may hide a failed child.
   Recheck mini idle state and collect its own backup on the mini. Stop its old
   app/host, replace the app, install the bundled host and launch it. Verify the
   same health/identity/data/account checks and actual MCP host roundtrip. Check
   its UI via the existing Screen Sharing connection when required by the tier.
   A local health check alone
   is insufficient. Keep both machines on the same accepted commit.
10. On either machine's failure, stop rollout. Keep the other machine's working
    version. Restore the failing machine's previous app/service; database migrations
    require the corresponding pre-update data snapshot before starting an older
    binary. Preserve failed-state data for diagnosis; never overwrite new user
    conversations. Report an unverified rollback as incomplete. MacBook failure
    must never trigger a Mac mini update.
11. Record per-machine receipts (commit, upstream commit, artifact/host hashes,
    environment, time, backup location and individual check outcomes), atomically
    under that machine's private support directory. Include incomplete/pending
    states; never turn a partial deployment into a no-op next day. Keep one
    known-good rollback archive, the latest build, and the permanent worktree.
    Archive only this run's temporary managed worktree after successful handoff.

A UI check should be read-only against existing production bots. Never run the
mutating Playwright suite against production data. A message-based MCP check
requires explicit authorization; read-only host-routed calls suffice for normal
post-update acceptance. Cloud assistants use account authorization AND the host's
own identity/registration/switch; the same login alone does not prove connectivity.
