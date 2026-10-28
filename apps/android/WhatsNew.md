<!--
Google Play release notes for the Android app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per Play language (zh-Hant, published as zh-TW;
en-US), listing what Android users notice, each under 500 characters (Play's limit;
tools/play-submit.py cuts longer notes at the last whole line). tools/play-submit.py reads the
released version's section; a version without one gets the PRs' "What's New: Android" bullets,
else a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.31.0

### zh-Hant
- 在「擴充套件」裡打開一個 Agent，登入下方會列出它自己的技能、聯結器、規則和外掛。
- Codex 的用量改成讀取目前登入帳號的額度，不會再停在舊的百分比。

### en-US
- Open an agent in Extensions. Under Sign in, see the skills, connectors, rules and plugins it already has.
- Codex usage now shows your signed-in account's current allowance instead of an old percentage.
