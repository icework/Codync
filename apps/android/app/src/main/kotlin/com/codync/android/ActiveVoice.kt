package com.codync.android

/** A call belongs to its original computer, even when another computer's chat is selected. */
internal fun activeVoiceState(states: List<AppState>): AppState? = states.firstOrNull { it.voice != null }
