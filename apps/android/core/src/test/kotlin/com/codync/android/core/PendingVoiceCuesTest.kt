package com.codync.android.core

import org.junit.Assert.*
import org.junit.Test

class PendingVoiceCuesTest {
    @Test fun sameHostReconnectsKeepCallsButIdentityAndProtocolChangesEndThem() {
        assertNull(voiceHostFailure("host-a", "host-a", false))
        assertTrue(voiceHostFailure("host-a", "host-b", false)!!.contains("identity changed"))
        assertTrue(voiceHostFailure("host-a", "host-a", true)!!.contains("compatible update"))
        assertNotNull(voiceHostFailure(null, "host-a", false))
    }

    @Test fun importantCuesSurviveAnEngineGapAndOnlyProgressIsReplaceable() {
        val pending = PendingVoiceCues(3)
        fun cue(kind: CallCue.Kind, seq: Long) = CallCue("call", kind, "value", seq)
        assertTrue(pending.append(cue(CallCue.Kind.Reply, 1)))
        assertTrue(pending.append(cue(CallCue.Kind.Progress, 2)))
        assertTrue(pending.append(cue(CallCue.Kind.Notice, 3)))
        assertTrue(pending.append(cue(CallCue.Kind.Progress, 4)))
        assertFalse(pending.append(cue(CallCue.Kind.Reply, 5)))
        val spoken = mutableListOf<Long>()
        var ready = true
        pending.flush(ready = { ready }, deliver = { spoken += it.seq; ready = false })
        assertEquals(listOf(1L), spoken)
        pending.flush(ready = { true }, deliver = { spoken += it.seq })
        assertEquals(listOf(1L, 3L, 4L), spoken)
        pending.flush(ready = { true }, deliver = { spoken += it.seq })
        assertEquals(listOf(1L, 3L, 4L), spoken)
    }
}
