<!--
App Store "What's New" for the iPhone app. Write it in the change that bumps the version:
a `## <version>` section with one `### <locale>` per App Store language (zh-Hant, en-US),
listing what iPhone users notice, without emoji (the App Store rejects them; asc-submit.py drops
any left). tools/asc-submit.py reads the released version's section;
a version without one gets a generic "Bug fixes and improvements" line. Delete old sections.
-->

## 2.39.1
### zh-Hant
- 語音通話不再因切換 bot 或返回列表而中斷；全域通話列可回到原 bot、靜音與掛斷。
- OpenAI 即時語音新增 GPT-Live 選項，可在交談時交辦工作、查詢進度與回覆審批；原有 Realtime 設定不變。
- GPT-Live 的模型測試會確認存取權限；語音與工具路由分別計費。
- 現在會在你從這支 iPhone 送出第五則訊息後，邀請你到 App Store 評分，一年最多兩次。
- 推理強度滑桿新增漸層點陣，讓不同強度更容易辨識。

### en-US
- Keep voice calls running while switching bots or returning to the list, with global controls for the original call.
- Choose GPT-Live for OpenAI voice calls while your bots handle tasks, status checks and approvals. Existing Realtime settings stay unchanged.
- GPT-Live model tests verify access without starting a paid call. Voice and tool routing are billed separately.
- Codync now invites you to rate it on the App Store after your fifth message from this iPhone, at most twice a year.
- The reasoning effort slider now uses a dotted gradient to make levels easier to distinguish.
