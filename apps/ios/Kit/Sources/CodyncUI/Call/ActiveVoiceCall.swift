import CodyncKit
import Foundation
import Observation
import SwiftUI

/// One device's call, pinned to its original computer independently of the selected conversation.
@MainActor @Observable public final class ActiveVoiceCall {
    struct Target: Identifiable {
        let id = UUID()
        let store: BotStore
        let botId: String
        let hostId: String

        @MainActor var failure: String? {
            if store.hostId != hostId { return "Voice chat ended because the computer identity changed. Start a new call." }
            if store.mismatch != nil { return "Voice chat ended because the app and computer need a compatible update." }
            return nil
        }
    }

    private(set) var target: Target?
    private(set) var ending: UUID?
    @ObservationIgnored private var mounted: UUID?
    var isSpeaking = false
    @ObservationIgnored var interrupt: (() -> Void)?

    public init() {}
    var canStart: Bool { target == nil && ending == nil }

    func start(store: BotStore, botId: String) {
        guard canStart, !store.retired, store.mismatch == nil,
              let hostId = store.hostId, store.bots[botId] != nil else { return }
        Dictation.stopActive()
        target = Target(store: store, botId: botId, hostId: hostId)
    }

    func isTarget(store: BotStore, botId: String) -> Bool {
        target?.store === store && target?.botId == botId
    }

    func started(_ id: UUID) -> Bool {
        guard target?.id == id, target?.failure == nil else { return false }
        mounted = id
        return true
    }

    func end() {
        guard let target else { return }
        ending = mounted == target.id ? target.id : nil
        self.target = nil
        isSpeaking = false
        interrupt = nil
    }

    func finished(_ id: UUID) {
        if ending == id { ending = nil }
        if mounted == id { mounted = nil }
    }
}

/// Mounted once above navigation, so popping a thread or switching tabs cannot tear down its audio.
public struct ActiveVoiceCallView: View {
    private let call: ActiveVoiceCall
    @Environment(AccountStore.self) private var accounts

    public init(call: ActiveVoiceCall) { self.call = call }

    public var body: some View {
        if let target = call.target {
            PersistentCall(target: target, call: call, accounts: accounts)
                .id(target.id)
                .padding(.vertical, 4)
                .transition(.move(edge: .top).combined(with: .opacity))
        }
    }
}

private struct PersistentCall: View {
    let target: ActiveVoiceCall.Target
    @Bindable var call: ActiveVoiceCall
    let accounts: AccountStore

    private var valid: Bool {
        !target.store.retired && accounts.store(for: target.store.computer.id) === target.store
            && target.store.bots[target.botId] != nil && target.failure == nil
    }

    var body: some View {
        CallView(botId: target.botId, isSpeaking: $call.isSpeaking, interrupt: $call.interrupt,
                 start: { call.started(target.id) }, showConversation: {
                     accounts.selection = BotReference(accountId: target.store.storage.accountID,
                         computerId: target.store.computer.id, botId: target.botId)
                 }, finished: { call.finished(target.id) }, close: { call.end() })
            .environment(target.store)
            .onChange(of: valid, initial: true) { _, valid in
                if !valid {
                    if let failure = target.failure { target.store.lastError = failure }
                    call.end()
                }
            }
    }
}
