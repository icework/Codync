import Foundation
import Testing
@testable import CodyncKit
@testable import CodyncUI

@MainActor
struct LiveCallTransportTests {
    private func event(_ type: String) -> Data { Data("{\"type\":\"\(type)\"}".utf8) }
    private func settle() async { for _ in 0..<30 { await Task.yield() } }

    @Test func onlyConversationLiveModelsUseTheNewTransport() {
        #expect(LiveCallTransport.isModel("gpt-live-1"))
        #expect(LiveCallTransport.isModel("gpt-live-1.1"))
        #expect(!LiveCallTransport.isModel("gpt-live-transcribe"))
        #expect(!LiveCallTransport.isModel("gpt-realtime-2.1"))
    }

    @Test func updatesWaitForSessionStartedAndProgressIsCoalesced() async {
        var updates: [String] = []
        var ready = 0
        var failures: [String] = []
        let bridge = LiveCallTransport(event: { _, _ in [] }, update: { id, kind, text in
            #expect(id == "session-1")
            updates.append("\(kind.rawValue):\(text)")
            return []
        }, send: { _ in }, ready: { ready += 1 }, failed: { failures.append($0) })
        bridge.attach("session-1")
        bridge.update(.progress, "old")
        bridge.update(.reply, "answer")
        bridge.update(.progress, "current")
        await settle()
        #expect(updates.isEmpty)
        bridge.receive(event("session.started"))
        bridge.receive(event("session.started"))
        await settle()
        #expect(ready == 1)
        #expect(updates == ["reply:answer", "progress:current"])
        #expect(failures.isEmpty)
    }

    @Test func forwardingPreservesOrderAndNeverForwardsAudio() async {
        var forwarded: [String] = []
        let bridge = LiveCallTransport(event: { id, data in
            #expect(id == "session-2")
            forwarded.append(String(decoding: data, as: UTF8.self))
            return []
        }, update: { _, _, _ in [] }, send: { _ in }, ready: {}, failed: { _ in })
        bridge.attach("session-2")
        for type in ["session.input_transcript.delta", "response.event", "session.output_audio.delta",
                     "session.output_transcript.delta", "session.delegation.created"] {
            bridge.receive(event(type))
        }
        await settle()
        #expect(forwarded == ["session.input_transcript.delta", "response.event", "session.output_transcript.delta",
                             "session.delegation.created"].map { String(decoding: event($0), as: UTF8.self) })
    }

    @Test func anOldSessionsLateToolResultCannotReachANewChannel() async {
        var result: CheckedContinuation<[Data], Error>?
        var sent: [Data] = []
        let bridge = LiveCallTransport(event: { _, _ in
            try await withCheckedThrowingContinuation { result = $0 }
        }, update: { _, _, _ in [] }, send: { sent.append($0) }, ready: {}, failed: { _ in })
        bridge.attach("old-session")
        bridge.receive(event("response.event"))
        await settle()
        #expect(result != nil)
        bridge.stop()
        result?.resume(returning: [event("response.create")])
        await settle()
        #expect(sent.isEmpty)
    }

    @Test func closeUsesLiveCommandAndAcceptsTerminalAcknowledgment() async {
        var sent: [Data] = []
        var failures: [String] = []
        let bridge = LiveCallTransport(event: { _, _ in [] }, update: { _, _, _ in [] },
            send: { sent.append($0) }, ready: {}, failed: { failures.append($0) })
        bridge.attach("session-3")
        bridge.requestClose()
        bridge.requestClose()
        bridge.receive(event("session.closed"))
        bridge.update(.reply, "too late")
        await settle()
        #expect(sent == [event("session.close")])
        #expect(bridge.isClosed)
        #expect(failures.isEmpty)
    }

    @Test func providerErrorsAreVisibleInsteadOfLeavingTheCallConnecting() {
        var failures: [String] = []
        let bridge = LiveCallTransport(event: { _, _ in [] }, update: { _, _, _ in [] },
            send: { _ in }, ready: {}, failed: { failures.append($0) })
        bridge.receive(Data(#"{"type":"error","error":{"message":"Model unavailable"}}"#.utf8))
        #expect(failures == ["Model unavailable"])
    }

    @Test func hangingUpDrainsReceivedTranscriptsWithoutStartingQueuedTools() async {
        var resume: CheckedContinuation<[Data], Error>?
        var forwarded: [String] = []
        var sent: [Data] = []
        let bridge = LiveCallTransport(event: { _, data in
            let type = String(decoding: data, as: UTF8.self)
            if data == event("response.event") {
                _ = try await withCheckedThrowingContinuation { resume = $0 }
            }
            forwarded.append(type)
            return [event("response.create")]
        }, update: { _, _, _ in [] }, send: { sent.append($0) }, ready: {}, failed: { _ in })
        bridge.attach("ending-session")
        bridge.receive(event("response.event"))
        await settle()
        bridge.receive(event("session.input_transcript.delta"))
        bridge.receive(event("response.event"))
        bridge.requestClose()
        bridge.receive(event("session.output_transcript.delta"))
        bridge.receive(event("session.closed"))
        #expect(forwarded.isEmpty)
        resume?.resume(returning: [])
        #expect(await bridge.drain())
        #expect(forwarded == ["response.event", "session.input_transcript.delta", "session.output_transcript.delta"].map { String(decoding: event($0), as: UTF8.self) })
        #expect(sent == [event("session.close")])
    }

    @Test func adjacentTranscriptDeltasOfOneSpeakerReachTheHostAsOneEvent() async throws {
        var forwarded: [[String: Any]] = []
        let bridge = LiveCallTransport(event: { _, data in
            forwarded.append(try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any]))
            return []
        }, update: { _, _, _ in [] }, send: { _ in }, ready: {}, failed: { _ in })
        bridge.attach("batched")
        bridge.receive(Data(#"{"type":"session.input_transcript.delta","event_id":"a","delta":"Open ","start_ms":0,"end_ms":10}"#.utf8))
        bridge.receive(Data(#"{"type":"session.input_transcript.delta","event_id":"b","delta":"app.ts","start_ms":10,"end_ms":20}"#.utf8))
        bridge.receive(Data(#"{"type":"session.output_transcript.delta","event_id":"c","delta":"Sure","start_ms":20,"end_ms":30}"#.utf8))
        bridge.receive(event("response.event"))
        await settle()
        #expect(forwarded.map { $0["type"] as? String } == ["session.input_transcript.delta", "session.output_transcript.delta", "response.event"])
        #expect(forwarded[0]["delta"] as? String == "Open app.ts")
        #expect(forwarded[0]["event_id"] as? String == "a")
        #expect(forwarded[0]["end_ms"] as? Int == 20)
        bridge.receive(Data(#"{"type":"session.output_transcript.delta","event_id":"d","delta":"Done","start_ms":40,"end_ms":50}"#.utf8))
        try await Task.sleep(for: .milliseconds(400))
        #expect(forwarded.count == 4, "a held delta goes out on its own once the batch window passes")
    }
}
