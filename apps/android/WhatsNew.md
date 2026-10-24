<!--
Google Play release notes for the Android app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per Play language (zh-Hant, published as zh-TW;
en-US), listing what Android users notice, each under 500 characters (Play's limit;
tools/play-submit.py cuts longer notes at the last whole line). tools/play-submit.py reads the
released version's section; a version without one gets the PRs' "What's New: Android" bullets,
else a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.25.0

### zh-Hant
- Bot 可以做 Artifact：報告、追蹤表、儀表板這類要回來看的內容會以卡片出現在聊天裡，點開看完整內容，直接在下方留言請它改，改好就自動更新。
- 每個 Bot 的「資料庫」收齊它做的 Artifact，以及聊天裡分享過的檔案和圖片。
- 可以建立使用 Hermes Agent 的 Bot。
- Bot 之間交辦工作不必等待，對方的回覆會回到發問的 Bot，再由它回覆你。
- 帳號選單裡的「電腦與設定」和它打開的頁面同名。
- 新增聯結器失敗後再試一次，會使用你改好的金鑰。

### en-US
- Bots can make artifacts: reports, trackers and dashboards you come back to show as a card in the chat. Open one full screen and message the bot below it to change it; it updates in place.
- Each bot's Library collects its artifacts and the files and images shared in the chat.
- Create bots that run Hermes Agent.
- Bots hand work to each other without waiting; the answer comes back to the bot that asked, which then replies to you.
- The account sheet's Computers & settings row now has the same name as the page it opens.
- Trying again after adding a connector failed uses the key you corrected.
