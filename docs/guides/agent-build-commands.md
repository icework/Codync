# Agent build and release reference

## Build, Test, and Development Commands

Run from the repository root unless a command changes directories:

- `xcodegen generate --spec apps/project.yml`: regenerate `apps/Codync.xcodeproj` after editing `apps/project.yml`; never edit `project.pbxproj` directly.
- Use Xcode’s `iOS Dev` scheme for development and `iOS` for production; see [Dev app isolation](../guides/dev-app.md).
- `cd apps/desktop && npm ci && npm run dev`: run the desktop app while developing and for its e2e check (renderer reloads on save, restart after `src/main` or preload changes; unpackaged, unsigned). In a linked worktree it runs a [sandbox](../guides/dev-app.md#worktree-sandboxes) beside the main checkout's Codync Dev; `--sandbox` does the same from the main checkout. In the main checkout, after a host change run `node tools/native-host.mjs dev` there first: `npm run dev` prefers that build over `host/target/debug`. Package `npm run dist:mac:dev` (about 45 s) only for what an unpackaged run lacks (bundled host install, Codync Screen / computer use, system permissions); `dist:*:main` only to check the production app. `npm run typecheck && npm test` for its CI checks.
- `cd host && cargo build`: build the host; `cargo run -- serve` starts it in the foreground.
- `cd host && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`: run host CI checks.
- `cd apps/android && ./gradlew :core:test :app:testDebugUnitTest :app:assembleDebug :app:lintDebug` (JDK 21, Android SDK 37): Android checks; `adb install -r -d app/build/outputs/apk/debug/app-debug.apk` installs Codync Dev.
- `cd apps/ios/Kit && xcodebuild test -scheme CodyncKit-Package -destination 'platform=iOS Simulator,name=<any iPhone>'`: run the iPhone package tests.
- `cd cloud && npm ci && npm test && npm run typecheck`: validate the cloud service; see `docs/guides/cloudflare-testing.md` for integration checks.
- `cd relay && npm ci && npm test && npm run typecheck`: install dependencies and validate the relay.

## Versioning

- Xcode Cloud archives the `iOS` scheme on every `v*` tag and uploads it to App Store Connect with its own build number, then the `Submit iOS` workflow submits it to App Review for the App Store; don't upload from the local machine or edit `CURRENT_PROJECT_VERSION` for it. Details: [development guide](../guides/development.md#ios-releases-xcode-cloud). `Submit Android` builds the same tags and uploads them to Google Play ([Android releases](../guides/development.md#android-releases-google-play)).
- Phone ↔ host compatibility: each side names the oldest version of the other it works with (`minApp` in the host's `hello`, `minHost` in each client). Additive changes need nothing; a rename/removal raises `minApp` in the same release, and the release gate holds that release until the App Store has the iPhone app: [docs/reference/compatibility.md](../reference/compatibility.md)
- **Every source change ships as a release.** A version bump on `main` is the only release trigger (Auto Tag → desktop app for macOS, Linux and Windows, host, Homebrew, in-app update), so any change to shipped code (`apps/`, `host/`, `packaging/`) bumps the version before it reaches `main`, without being asked. Docs, `web/`, `cloud/`, `relay/` and CI-only changes don't bump.
- **`cloud/` is deployed by hand, never by CI.** A dev → main PR that changes `cloud/` first runs `npm run deploy:dev` and is tested end to end on dev; right after it merges, run `npm run deploy:main` (plus any new bucket/migration) and check the new routes, so dev never works while main fails: [releasing cloud changes](../guides/environments-and-deployment.md#releasing-cloud-changes).
- Pick the bump yourself: patch for fixes and small tweaks, minor for new features or protocol additions. Never bump major (stay on 2.x). One bump per merge into `main`: if `MARKETING_VERSION` is already ahead of the latest `v*` tag, leave it. A change to `host/crates/memory` or `host/crates/network` also raises that crate's own 0.x version (`codync-mem`, `codync-network`) once per release (leave it if it's already ahead of `main`'s); CI blocks a PR into `main` that forgets, and the merge mirrors and publishes it to crates.io ([crate releases](../guides/development.md#codync-mem-releases)).
- How: set `MARKETING_VERSION` in `apps/project.yml` and `version` in `host/Cargo.toml` (+ its `Cargo.lock` entry) and `apps/desktop/package.json` (+ `package-lock.json`) to the same value, run `xcodegen generate --spec apps/project.yml`, commit as `build: bump version to X.Y.Z`. Never touch `CURRENT_PROJECT_VERSION` (Xcode Cloud sets the iOS build number). When a phone app changes, also write that version's section in its notes (zh-Hant + en-US, what that app's users notice): `apps/ios/WhatsNew.md` becomes the App Store "What's New", `apps/android/WhatsNew.md` the Google Play release notes.

