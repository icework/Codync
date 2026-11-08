import CodyncKit
import Foundation
import os

/// Ordered forwarding between the Live data channel and the host. No tool or transcript logic runs here.
@MainActor
final class LiveCallTransport {
    private let log = Logger(subsystem: "com.pokai.Codync", category: "LiveCallTransport")
    typealias EventRequest = @MainActor (String, Data) async throws -> [Data]
    typealias UpdateRequest = @MainActor (String, CallCue.Kind, String) async throws -> [Data]
    private let eventRequest: EventRequest
    private let updateRequest: UpdateRequest
    private let send: (Data) -> Void
    private let ready: () -> Void
    private let failed: (String) -> Void
    private var queue: Task<Void, Never>?
    private var queued = 0
    private var stopped = false
    private var closing = false
    private var started = false
    private var pending: [(CallCue.Kind, String)] = []
    /// Live sends a transcript delta about every word: adjacent ones of one speaker reach the host as one event.
    private var held: (message: [String: Any], data: Data)?
    private var heldTimer: Task<Void, Never>?
    private static let transcriptBatch = Duration.milliseconds(250)
    private static let transcriptBatchChars = 4_000
    private(set) var sessionId: String?
    private(set) var isClosed = false

    static func isModel(_ model: String) -> Bool {
        model.hasPrefix("gpt-live-") && !model.contains("transcribe")
    }

    init(event: @escaping EventRequest, update: @escaping UpdateRequest,
         send: @escaping (Data) -> Void, ready: @escaping () -> Void, failed: @escaping (String) -> Void) {
        eventRequest = event
        updateRequest = update
        self.send = send
        self.ready = ready
        self.failed = failed
    }

    convenience init(link: CallLink, send: @escaping (Data) -> Void,
                     ready: @escaping () -> Void, failed: @escaping (String) -> Void) {
        self.init(event: { session, event in
            try await link.client().callLiveEvent(call: link.call, sessionId: session, event: event)
        }, update: { session, kind, text in
            try await link.client().callLiveUpdate(call: link.call, sessionId: session, kind: kind, text: text)
        }, send: send, ready: ready, failed: failed)
    }

    func attach(_ sessionId: String) { self.sessionId = sessionId }

    func receive(_ data: Data) {
        guard let message = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
              let type = message["type"] as? String else { return }
        if type == "session.closed" {
            flushTranscript()
            isClosed = true
            if !closing && !stopped { failed("The GPT-Live session ended.") }
            return
        }
        let transcript = type == "session.input_transcript.delta" || type == "session.output_transcript.delta"
        guard !stopped, !closing || transcript else { return }
        if type == "error" || type == "session.error" {
            let error = message["error"] as? [String: Any]
            failed(error?["message"] as? String ?? "GPT-Live could not continue the call.")
            return
        }
        if type == "session.started" {
            guard !started else { return }
            started = true
            ready()
            let waiting = pending
            pending.removeAll()
            for (kind, text) in waiting { update(kind, text) }
            return
        }
        guard ["response.event", "session.input_transcript.delta", "session.output_transcript.delta",
               "session.delegation.created"].contains(type), let sessionId else { return }
        if transcript { return hold(message, data, type: type) }
        flushTranscript()
        enqueue { [eventRequest] in try await eventRequest(sessionId, data) }
    }

    func update(_ kind: CallCue.Kind, _ text: String) {
        guard !stopped, !closing else { return }
        guard started, let sessionId else {
            if kind == .progress { pending.removeAll { $0.0 == .progress } }
            guard pending.count < 32 else { return failed("Too many voice updates are waiting for GPT-Live.") }
            pending.append((kind, text))
            return
        }
        flushTranscript()
        enqueue { [updateRequest] in try await updateRequest(sessionId, kind, text) }
    }

    /// Stop accepting work immediately; a result from the old session can never reach its replacement.
    func stop() {
        stopped = true
        queue?.cancel()
        queue = nil
        pending.removeAll()
        heldTimer?.cancel()
        heldTimer = nil
        held = nil
    }

    func requestClose() {
        guard !closing, !isClosed else { return }
        closing = true
        pending.removeAll()
        send(Data(#"{"type":"session.close"}"#.utf8))
    }

    func drain() async -> Bool {
        flushTranscript()
        let deadline = ContinuousClock.now + .seconds(2)
        while queued > 0, ContinuousClock.now < deadline {
            try? await Task.sleep(for: .milliseconds(20))
        }
        return queued == 0
    }

    /// Joins a transcript delta onto the held one when it continues it; it waits at most `transcriptBatch`.
    private func hold(_ message: [String: Any], _ data: Data, type: String) {
        if let current = held, current.message["type"] as? String == type,
           let joined = current.message["delta"] as? String, let delta = message["delta"] as? String,
           joined.count + delta.count <= Self.transcriptBatchChars {
            var merged = current.message
            merged["delta"] = joined + delta
            merged["end_ms"] = message["end_ms"]
            if let encoded = try? JSONSerialization.data(withJSONObject: merged) {
                held = (merged, encoded)
                return
            }
        }
        flushTranscript()
        held = (message, data)
        heldTimer = Task { [weak self] in
            try? await Task.sleep(for: Self.transcriptBatch)
            guard !Task.isCancelled else { return }
            self?.flushTranscript()
        }
    }

    private func flushTranscript() {
        heldTimer?.cancel()
        heldTimer = nil
        guard let data = held?.data, let sessionId else { return }
        held = nil
        enqueue(transcript: true) { [eventRequest] in try await eventRequest(sessionId, data) }
    }

    private func enqueue(transcript: Bool = false, _ operation: @escaping @MainActor () async throws -> [Data]) {
        guard queued < 256 else { return failed("The computer cannot keep up with GPT-Live events.") }
        queued += 1
        let previous = queue
        queue = Task { [weak self] in
            await previous?.value
            guard let self else { return }
            defer { self.queued -= 1 }
            guard !Task.isCancelled, !self.stopped, !self.closing || transcript else { return }
            do {
                let commands = try await operation()
                guard !Task.isCancelled, !self.stopped, !self.closing else { return }
                for command in commands { self.send(command) }
            } catch {
                if self.closing && transcript { self.log.error("GPT-Live transcript delivery failed while hanging up.") }
                guard !Task.isCancelled, !self.stopped, !self.closing else { return }
                self.failed(error.localizedDescription)
            }
        }
    }
}
