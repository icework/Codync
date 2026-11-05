# Personal macOS build

Kai's `codex/personal` branch tracks upstream `origin/dev` and is backed up to
`fork/codex/personal` (`icework/Codync`). `origin` remains `leepokai/Codync`.
Never push personal changes to upstream `dev` or `main`.

The persistent worktree is `/Users/kaiwu/.codex/worktrees/codync-personal/Codync`.
The shared checkout `/Users/kaiwu/Projects/Codync` remains on upstream `dev`.
The installed app is `/Applications/Codync Dev.app`, bundle `com.pokai.Codync.dev`,
with its host on 19223 and existing data at `~/.codync-dev`. Production stays on
19222. Never copy production credentials or identities into Dev.

## Personal icon and build

`apps/desktop/resources/PersonalAppIcon.icon` preserves the upstream mark and
adds a vector mint DEV badge. Only the macOS Dev packaging override selects it.
This is a personal macOS distribution distinction, not an in-app UI change:
iOS, Android, Linux, Windows and the terminal keep their existing behavior and
icons. A future upstream mark change should update the copied mark too.

Use the upstream build pipeline with this Mac's installed Apple Development
identity supplied at packaging time (not committed). The upstream npm script
names the maintainer's signing identity, so override the explicit mac identity:

```sh
cd apps/desktop
export PATH="$HOME/.cargo/bin:$PATH"
npm ci
npm run typecheck
npm test
# The upstream Dev script does not build this optional speech helper.
# Build it explicitly so the personal app retains on-device dictation.
swift build -c release --package-path native/speech-macos --scratch-path ../../build/dd/speech-macos
mkdir -p build/native
cp ../../build/dd/speech-macos/release/codync-speech build/native/codync-speech
npm run dist:mac:dev -- --config.mac.identity='<local signing identity>'
```

Find the identity with `security find-identity -v -p codesigning`. Do not publish
a release or change the upstream release workflow. The current version is
already ahead of the latest release tag; future version changes follow the
repository rule, once per upstream release, never a daily artificial bump.

## Daily update procedure

The Codex scheduled task runs daily at 04:00 America/Los_Angeles. It has standing
authorization to fetch, merge conflict-free upstream updates, validate, commit,
push the personal branch to the fork, and replace this Mac's Dev app. Keep the
persistent worktree; do not archive it at the end of a run. Scheduling requires
the local computer and Codex desktop app to be available.

1. Acquire a single-run lock in `~/Library/Application Support/Codync Personal/`.
   Record the owning PID. Release only a lock owned by this run; inspect a stale
   lock before recovery. Read the current repository instructions and this guide.
   Inspect the personal worktree, fork branch and installed receipt. Do not stash,
   discard, reset or overwrite another task's edits. If the branch is dirty or
   busy, report that blocker and leave the installed app alone.
2. Fetch `origin` and `fork`. Fast-forward personal from its fork counterpart only
   when safe; stop for divergent personal histories. Do not push to `origin`.
   If origin/dev is already an ancestor of personal and the installed receipt
   matches the personal commit and bundled hashes, and its required checks are
   complete, no rebuild is needed. An initial installation awaiting login is not
   a fully validated deployment.
3. Create an isolated candidate worktree from personal with the managed worktree
   tool, then a `codex/sync-<timestamp>` branch. Merge `origin/dev` there. Review the
   incoming diff, especially packaging, migrations and app/host compatibility.
   Keep the personal icon and Dev isolation. Use merge, not a history rewrite.
4. On a conflict, leave personal and the installed app unchanged. Save a conflict
   report with files, both sides' intent, actual conflicting hunks and affected
   behavior. Offer: preserve personal behavior while adapting to upstream;
   adopt upstream behavior and retire the personal override; or defer this update.
   Recommend an option per conflict. Wait for Kai's decision; never use blanket
   ours/theirs, force push, or silently discard either side. Preserve the candidate
   worktree for inspection. If the same unresolved update recurs, reuse the report.
5. For a clean merge, run desktop typecheck and tests, affected host checks, and
   applicable checks for other changed modules. Follow after-change for newly
   authored fixes. Build and run the worktree sandbox and the full Playwright
   suite. Managed worktrees share the basename `Codync`, so use a unique name
   in both commands, for example `npm run dev -- --name personal-check --cdp`
   and `CODYNC_SANDBOX_NAME=personal-check npm run e2e` (use a timestamp suffix
   for daily candidates). Never reuse a name belonging to another worktree.
   Missing checks are blockers, not passes. If checks fail, report evidence
   and keep the installed app and personal branch unchanged. Stop only this run's
   sandbox after testing.
6. Before packaging/installing, record the old app/host state, quiesce only the
   installed Dev variant and wait for it to exit. Never stop production, another
   worktree, or interrupt active bot turns: defer deployment if Dev is busy.
   Back up Dev data, desktop settings, launch-agent plist and the previous app
   (as an archive, not another launchable .app) under the private support folder.
   Exclude transient sockets from data archives; verify each archive before installation.
   Use mode 0700 on the backup directory and 0600 on archives. Never write credentials into logs/Git.
   Apple build output stays under the candidate's `build/dd`.
7. Package with the local signing identity; on build failure restart the previous
   Dev service/app and report failure. Check signature, Dev bundle/environment,
   icon and bundled host. Confirm personal has not moved since step 3, then
   fast-forward personal to the validated candidate and push **only**
   `fork codex/personal`. A failed push blocks installation; restore the old
   running app if it was stopped, retain the candidate and explain the failure.
8. Replace the entire installed bundle as in [development](development.md#replace-an-installed-macos-build-completely).
   Run its new bundled host's `install --port 19223`, launch the app and verify
   `/health` version, environment, binary path and SHA-256, plus the connected
   main screen using Computer Use. Keep the Dock item pointing at the fixed app
   path. The Dock badge is part of the signed bundle, not a fragile Finder override.
9. On installation/startup failure, stop only the failed Dev variant and restore
   the previous bundle/service. Data migrations need the matching pre-update data
   backup; do not run an older binary on migrated data. Report deployed failure
   separately from Git merge success. Do not claim an unverified rollback succeeded.
10. On success, atomically write a local `installed.json` receipt in the support
    folder with personal/upstream commits, version, installed path, host SHA-256,
    timestamp and check results. Record a timestamped Chinese run report. Clean
    up only this run's temporary worktree/processes after successful push and
    verification. Retain the persistent personal worktree, latest build and latest
    known-good rollback archive. Avoid duplicate launchable app bundles.

The merge, validation and deployment are distinct outcomes. A failed deployment
may leave the branch ahead of the running app; the receipt makes that visible and
the next run must retry deployment rather than declaring no changes.
