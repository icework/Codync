package com.codync.android.core

/** Cues belong to the call, not the replaceable engine. Fail visibly rather than drop an approval. */
class PendingVoiceCues(private val limit: Int = 128) {
    private val cues = ArrayDeque<CallCue>()
    fun append(cue: CallCue): Boolean {
        if (cue.kind == CallCue.Kind.Progress) cues.removeAll { it.kind == CallCue.Kind.Progress }
        if (cues.size >= limit) return false
        cues.addLast(cue)
        return true
    }
    fun flush(ready: () -> Boolean, deliver: (CallCue) -> Unit) {
        while (cues.isNotEmpty() && ready()) deliver(cues.removeFirst())
    }
}
