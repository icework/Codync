import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

@MainActor @Test func activeCallKeepsItsOriginalComputerWhileNavigationChanges() throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let first = BotStore(computer: randomComputer("First"), storage: storage)
    let second = BotStore(computer: randomComputer("Second"), storage: storage)
    defer { first.retire(); second.retire() }
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"shared-id","name":"Soya","rev":1}"#.utf8))
    first.hostId = "host-first"
    second.hostId = "host-second"
    first.bots[bot.id] = bot
    second.bots[bot.id] = bot
    let call = ActiveVoiceCall()
    call.start(store: first, botId: bot.id)
    let id = try #require(call.target?.id)
    #expect(call.started(id))
    first.selection = nil
    second.selection = bot.id
    call.start(store: second, botId: bot.id)
    #expect(call.target?.id == id)
    #expect(call.isTarget(store: first, botId: bot.id))
    #expect(!call.isTarget(store: second, botId: bot.id))
    call.end()
    #expect(call.target == nil)
    #expect(!call.canStart)
    call.finished(UUID())
    #expect(!call.canStart)
    call.finished(id)
    #expect(call.canStart)
    call.start(store: second, botId: bot.id)
    #expect(call.isTarget(store: second, botId: bot.id))
}

@MainActor @Test func callsCannotStartOnMissingBotsOrRetiredComputers() throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let store = BotStore(computer: randomComputer("Mac"), storage: storage)
    let call = ActiveVoiceCall()
    call.start(store: store, botId: "missing")
    #expect(call.canStart)
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","name":"Soya","rev":1}"#.utf8))
    store.hostId = "original-host"
    store.bots[bot.id] = bot
    store.retire()
    call.start(store: store, botId: bot.id)
    #expect(call.canStart)
}


@MainActor @Test func pendingCallCuesKeepRepliesAcrossEngineChangesAndCoalesceProgress() throws {
    func cue(_ kind: String, _ seq: Int) throws -> CallCue {
        try JSONDecoder().decode(CallCue.self, from: Data("{\"call\":\"c\",\"kind\":\"\(kind)\",\"text\":\"value\",\"seq\":\(seq)}".utf8))
    }
    let pending = PendingCallCues(limit: 3)
    #expect(pending.append(try cue("reply", 1)))
    #expect(pending.append(try cue("progress", 2)))
    #expect(pending.append(try cue("notice", 3)))
    #expect(pending.append(try cue("progress", 4)))
    #expect(!pending.append(try cue("reply", 5)))
    var spoken: [Int64] = []
    var ready = true
    pending.flush(ready: { ready }, deliver: { spoken.append($0.seq); ready = false })
    #expect(spoken == [1])
    pending.flush(ready: { true }, deliver: { spoken.append($0.seq) })
    #expect(spoken == [1, 3, 4])
    pending.flush(ready: { true }, deliver: { spoken.append($0.seq) })
    #expect(spoken == [1, 3, 4])
}


@MainActor @Test func cancellingBeforeTheCallViewMountsDoesNotBlockTheNextCall() throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let store = BotStore(computer: randomComputer("Mac"), storage: storage)
    defer { store.retire() }
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","name":"Soya","rev":1}"#.utf8))
    store.hostId = "original-host"
    store.bots[bot.id] = bot
    let call = ActiveVoiceCall()
    call.start(store: store, botId: bot.id)
    let cancelled = try #require(call.target?.id)
    call.end()
    #expect(call.canStart)
    #expect(!call.started(cancelled))
    call.start(store: store, botId: bot.id)
    #expect(call.target?.id != cancelled)
    #expect(!call.canStart)
}


@MainActor @Test func callsRejectChangedHostIdentityAndProtocolButKeepSameHostReconnects() throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let store = BotStore(computer: randomComputer("Mac"), storage: storage)
    defer { store.retire() }
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","name":"Soya","rev":1}"#.utf8))
    store.hostId = "original-host"
    store.bots[bot.id] = bot
    let call = ActiveVoiceCall()
    call.start(store: store, botId: bot.id)
    let target = try #require(call.target)
    #expect(target.failure == nil)
    store.hostId = "original-host"
    #expect(target.failure == nil)
    store.hostId = "different-host"
    #expect(target.failure?.contains("identity changed") == true)
    store.hostId = "original-host"
    store.hostVersion = HostVersion(version: "99.0.0", minApp: "99.0.0")
    #expect(target.failure?.contains("compatible update") == true)
    call.end()
    call.start(store: store, botId: bot.id)
    #expect(call.target == nil)
}


@MainActor @Test func callRequestsWaitForThisConnectionsHelloAndRejectANewIdentity() async throws {
    let (storage, suite) = context()
    defer { UserDefaults(suiteName: suite)?.removePersistentDomain(forName: suite) }
    let fake = FakeRemote(.ready(.direct))
    await fake.holdHello(true)
    let store = BotStore(computer: randomComputer("Mac"), storage: storage) { _ in fake }
    defer { store.retire() }
    store.hostId = "h1"
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","name":"Lead","rev":1,"lead":true}"#.utf8))
    store.bots[bot.id] = bot
    let link = CallLink(model: store, botId: bot.id, call: "call")
    store.setActive(true)
    #expect(await until { await fake.helloWaiting })
    #expect(store.client != nil)
    #expect(store.live == nil)
    let request = Task { try await link.start("device") }
    defer { request.cancel() }
    store.callStarted(bot.id, call: "call", expectedHostId: "h1")
    #expect(await until { store.waiting >= 2 })
    #expect(await fake.calls.filter { $0 == "startCall" || $0 == "callStarted" }.isEmpty)
    await fake.setHello(#"{"hostId":"h2","name":"Mac","version":"3.0.0","os":"macos","backends":[],"rev":0}"#)
    await fake.holdHello(false)
    #expect(await until { await fake.subscribed })
    await fake.emit(#"{"type":"hello","hostId":"h2","rev":0}"#)
    #expect(await until { store.connection == .online })
    do {
        _ = try await request.value
        Issue.record("An old call reached the new host")
    } catch { #expect(error.localizedDescription.contains("identity")) }
    #expect(await until { store.waiting == 0 })
    #expect(await fake.calls.filter { $0 == "startCall" || $0 == "callStarted" }.isEmpty)
    #expect(store.lastError?.contains("identity") == true)
    store.restartStream()
    #expect(store.live == nil)
}
