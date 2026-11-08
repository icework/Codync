<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.40.2
### zh-Hant
- 語音通話不再因切換 bot 或返回列表而中斷；全域通話列可回到原 bot、靜音與掛斷。
- OpenAI 即時語音新增 GPT-Live 選項，可在交談時交辦工作、查詢進度與回覆審批；原有 Realtime 設定不變。
- GPT-Live 的模型測試會確認存取權限；語音與工具路由分別計費。
- 現在會在你從這支 iPhone 送出第五則訊息後，邀請你到 App Store 評分，一年最多兩次。
- 推理強度滑桿新增漸層點陣，讓不同強度更容易辨識。
- bot 現在可以讓 dev server 持續在電腦上執行，不會因為 bot 閒置或重新啟動而中斷。port 會顯示在聊天標題下方；點 bot 的名稱，就能看記錄、重新啟動或停止；手機連得到電腦時，也能直接開啟。
- 用量頁面可以自己選要看哪些服務並調整順序：預設只顯示 bot 正在用的，也可以打開或隱藏其他服務，或拖曳排列。另外支援顯示 Google Antigravity 和 GitHub Copilot 的額度。
- 修正 bot 更新時，選單偶爾點了沒反應的問題。

### en-US
- Keep voice calls running while switching bots or returning to the list, with global controls for the original call.
- Choose GPT-Live for OpenAI voice calls while your bots handle tasks, status checks and approvals. Existing Realtime settings stay unchanged.
- GPT-Live model tests verify access without starting a paid call. Voice and tool routing are billed separately.
- Codync now invites you to rate it on the App Store after your fifth message from this iPhone, at most twice a year.
- The reasoning effort slider now uses a dotted gradient to make levels easier to distinguish.
- Bots can now keep a dev server running on your computer, even when the bot goes idle or restarts. Its port shows under the chat title; tap the bot's name to read its logs, restart or stop it, and to open it when your phone can reach it.
- Choose and arrange which agents show in Usage: by default it lists the ones your bots run, and you can turn others on, hide them or drag them into order. Usage now also reads Google Antigravity's and GitHub Copilot's limits.
- Fixed menus sometimes ignoring a tap while a bot was updating.
