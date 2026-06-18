package com.avinthakur080.pulse_rs

import android.content.Intent
import android.app.Activity

object ShareBridge {
    external fun onShareUrl(url: String)

    @JvmStatic
    fun startShareIntent(activity: Activity, title: String, url: String) {
        val text = if (url.isEmpty()) title else "$title\n$url"
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, title)
            putExtra(Intent.EXTRA_TEXT, text)
        }
        activity.startActivity(Intent.createChooser(intent, "Share via"))
    }
}
