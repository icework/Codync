package com.codync.android.core

/** A reconnect may replace the transport, but must never retarget an existing call. */
fun voiceHostFailure(original: String?, current: String?, incompatible: Boolean): String? = when {
    original == null || original != current -> "Voice chat ended because the computer identity changed. Start a new call."
    incompatible -> "Voice chat ended because the app and computer need a compatible update."
    else -> null
}
