# GPT Live 个人分支合并验收

## 范围与结果

用户选择“保留双方功能并适配完整合并”。候选分支 `codex/personal-gpt-live`，基线 `a129d7b21a0c60c0f0b38926c049d7420fcf0d82`，合入 `fork/codex/gpt-live-api` 的 `5e326c1fb647106aff5db0ce66b8ac236fc21fa2`。17 个冲突文件已逐项解决，未采用全量 ours/theirs。只向个人 fork 推送，不向上游提交；本次不部署或重启两台生产 App。

保留现有听写模式、翻译、Hub 配对/模型缓存初始化、context 测试、个人更新保护和每日/每周维护策略。Xcode 工程由 xcodegen 生成。版本保持 2.39.1（已领先最新 v2.39.0），两种手机的双语说明均加入本次功能。

## 独立审查与修复

三个只读 reviewer 完成 correctness/completeness/refactor 审查，另由 verifier 核实问题。已解决：

1. 桌面/iOS 挂断可能越过 Live 转写队列：增加有界结束屏障，先静音/关麦，再收尾已收到转写，之后才 endCall；iOS 保留 beginCallEnd 连接保活。引擎切换和失败回退也等待旧会话收尾，并拒绝过期切换。
2. Android 慢 Host RPC 会阻塞静音/失焦：改为容量 256 的顺序 Host 队列，与本地控制分离；重连/关闭校验 generation，旧结果不会回放。补回归测试覆盖延迟请求期间静音、失焦与挂断保存。
3. Live 使用与故障不能在 analytics 区分：三端及 Host 白名单增加受限 `openai_live` 引擎值，沿用原事件，不发送模型名、转写、SDP、会话 ID 或错误正文。

重构审查没有额外建议。复审确认上述问题和引擎切换遗漏均已修复。

## 验证证据

- Desktop typecheck 通过，完整单元测试 413 通过；新增转写队列测试通过。
- Host 完整 cargo test：868 通过、1 ignored；fmt 与 all-target clippy `-D warnings` 通过。埋点变更后 4 项相关测试通过，并再次运行 clippy。
- iOS CodyncKit-Package 模拟器测试：387 通过（CodyncUI 193、CodyncKit 194）；之后针对 Live/CallRecovery 与最终 Live transport 改动补跑相关测试。缺失的 Apple Metal toolchain 已通过 Xcode 官方命令补齐。
- Android core：229 项中 228 通过、1 skipped。使用从官方 Adoptium 发布下载并校验 SHA256 的临时 JDK21，未修改系统默认 Java。
- 桌面 Playwright 全套：10 通过、1 跳过（沙箱 cloud off）。最终受影响 call/settings 流程 3 通过，其中新增挂断屏障检查验证 transcript 在 endCall 前完成。
- GPT Live 设置 E2E 使用真实 Electron 界面和供应商模型元数据 fixture；没有使用真实 Key 或发起付费通话。转写/关闭顺序测试同样使用协议 fixture，不能当成真实声学测试。

## 未验收及部署边界

- Android App 的 build/unit/lint 因 Android SDK 缺失无法执行；核心模块编译及测试通过不等于 APK 构建通过。
- Maestro 和规定的共享 Android 模拟器不可用，未进行 iOS/Android 手机界面 E2E。没有新建替代 Android AVD。
- 未验证本次合并后的真实麦克风 GPT Live 呼叫、真实音频播放或 PostHog 到达。
- Linux/Windows VM 不可用，未执行这些平台 E2E。
- 原分支随合并保留的 2026-10-28 历史截图/报告只代表历史证据，不能代替本次验证。
- 本次是源码合并，生产安装收据不改。下一次部署按个人版完整验证规则补齐所需验收后，才可先 MacBook、后 Mac mini 更新。

## 原版 iOS 兼容性

Host 的最低客户端版本仍是 2.36.0，没有因 GPT Live 提高。旧客户端没有 supportsLive 能力声明时，Host 不提供 Live 模型，也不把 Live 设成 Realtime 默认。原版 iOS 可继续现有聊天及 Realtime 语音；GPT Live 需要更新后的客户端语音连接/播放/事件处理。Agent、工具与权限逻辑仍在 Host，客户端只处理设备音频和传输。

参考：[OpenAI WebRTC](https://developers.openai.com/api/docs/guides/voice-webrtc)、[GPT Live 委派与工具](https://developers.openai.com/api/docs/guides/live-delegation)。
