package com.avinthakur080.pulse_rs

import android.content.Intent
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        handleShareIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleShareIntent(intent)
    }

    private fun handleShareIntent(intent: Intent?) {
        if (intent == null) return
        val url: String? = when (intent.action) {
            Intent.ACTION_SEND ->
                if (intent.type == "text/plain") intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()
                else null
            Intent.ACTION_VIEW -> intent.data?.toString()?.trim()
            else -> null
        }
        if (!url.isNullOrBlank() && (url.startsWith("http://") || url.startsWith("https://"))) {
            ShareBridge.onShareUrl(url)
        }
    }
}
