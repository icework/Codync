import CodyncKit

/// A call keeps important cues while its audio engine connects or changes. Progress is replaceable.
@MainActor final class PendingCallCues {
    private var cues: [CallCue] = []
    private let limit: Int
    init(limit: Int = 128) { self.limit = limit }

    func append(_ cue: CallCue) -> Bool {
        if cue.kind == .progress { cues.removeAll { $0.kind == .progress } }
        guard cues.count < limit else { return false }
        cues.append(cue)
        return true
    }

    func flush(ready: () -> Bool, deliver: (CallCue) -> Void) {
        while !cues.isEmpty && ready() { deliver(cues.removeFirst()) }
    }
}
