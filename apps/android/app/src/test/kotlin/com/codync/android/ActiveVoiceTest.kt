package com.codync.android

import com.codync.android.core.Computer
import org.junit.Assert.*
import org.junit.Test

class ActiveVoiceTest {
    @Test fun changingSelectedComputerDoesNotRetargetTheCallControls() {
        val owner = AppState(Computer("first", "First", "key"), selectedBot = null,
            voice = VoiceCallViewState("call", "same-bot-id"))
        val selected = AppState(Computer("second", "Second", "key"), selectedBot = "same-bot-id")
        assertSame(owner, activeVoiceState(listOf(selected, owner)))
        assertEquals("call", activeVoiceState(listOf(selected, owner))?.voice?.id)
        assertNull(activeVoiceState(listOf(selected, owner.copy(voice = null))))
    }
}
