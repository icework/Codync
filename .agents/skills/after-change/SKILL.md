---
name: after-change
description: Review, fix, clean up and E2E-test our Codync changes; personal work uses focused Terra testing. External issues/PRs use the issue-PR review guide.
---

# After change

Resolve the task worktree and its applicable AGENTS.md. Record exact base/head,
owned diff and already valid checks; never reset or stash another agent's work.

For personal work (`codex/personal`, derived task branches or fleet maintenance),
read [efficient personal testing](../../../docs/guides/personal-testing.md) and
follow it instead of generic review-count or whole-suite defaults:

1. Docs-only: diff/link/artifact validation by one fresh-context `gpt-6.1-terra`
   worker. No app build/E2E, production restart or three-reviewer pipeline.
2. Runtime: choose focused/expanded/weekly once; Terra runs selected commands and
   changed-flow sandbox E2E, preserving full logs locally and returning <=8 lines.
3. One independent reviewer covers correctness/completeness/cleanup; coordinator
   verifies findings. Add a specialist only for a concrete unresolved risk.
4. Fix confirmed issues, rerun only invalidated checks once, report failures and
   pending coverage honestly; then follow authorized personal integration/deployment.
5. Stop and clean only owned test processes/sandboxes after successful handoff.
   Preserve protected/shared worktrees and all production identities/data.

Other upstream work follows [upstream workflow](upstream-workflow.md); load it
only when applicable. External contributions still follow
[issue and PR review](../../../docs/guides/issue-pr-review.md).
