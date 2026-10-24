import AppIntents
import CodyncKit
import CodyncUI

struct OpenLiveModeIntent: AppIntent {
    static let title: LocalizedStringResource = "Open Live Mode"
    static let description = IntentDescription("Open a bot's conversation and start voice chat. Connect a computer in Codync first.")
    static let openAppWhenRun = true

    @Parameter(title: "Bot", requestValueDialog: "Which bot would you like to talk to?")
    var bot: ShortcutBotEntity

    static var parameterSummary: some ParameterSummary {
        Summary("Open Live Mode with \(\.$bot)")
    }

    @MainActor func perform() async throws -> some IntentResult {
        let store = try AppStore.shared.openShortcutBot(bot)
        guard let ref = bot.reference(in: AppStore.shared.storage) else { throw HostError.unauthorized("Choose a bot from the current account.") }
        try await store.requestVoiceCall(to: ref.botId)
        return .result()
    }
}

struct SendTextToBotIntent: AppIntent {
    static let title: LocalizedStringResource = "Send Text to Bot"
    static let description = IntentDescription("Send text to a bot and open its conversation. Finishes when the computer or its offline mailbox accepts the message.")
    static let openAppWhenRun = true

    @Parameter(title: "Bot", requestValueDialog: "Which bot should receive the text?")
    var bot: ShortcutBotEntity

    @Parameter(title: "Text", requestValueDialog: "What would you like to send?")
    var text: String

    static var parameterSummary: some ParameterSummary {
        Summary("Send \(\.$text) to \(\.$bot)")
    }

    @MainActor func perform() async throws -> some IntentResult & ProvidesDialog {
        let store = try AppStore.shared.openShortcutBot(bot)
        guard let ref = bot.reference(in: AppStore.shared.storage) else { throw HostError.unauthorized("Choose a bot from the current account.") }
        let progress = try await store.sendAndWait(text, to: ref.botId)
        if case .queued = progress { return .result(dialog: "Message queued for the offline computer.") }
        return .result(dialog: "Message sent.")
    }
}

struct BotShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(intent: OpenLiveModeIntent(), phrases: ["Open Live Mode in \(.applicationName)"],
                    shortTitle: "Open Live Mode", systemImageName: "waveform")
        AppShortcut(intent: SendTextToBotIntent(), phrases: ["Send text with \(.applicationName)"],
                    shortTitle: "Send Text to Bot", systemImageName: "paperplane")
    }
}
