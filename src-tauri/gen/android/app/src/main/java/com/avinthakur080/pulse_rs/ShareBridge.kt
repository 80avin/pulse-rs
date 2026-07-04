package com.avinthakur080.pulse_rs

import android.app.Activity
import android.content.Intent
import androidx.core.content.FileProvider
import androidx.annotation.Keep

@Keep // Prevents ProGuard from stripping this class and its methods
object ShareBridge {
    external fun init(context: android.content.Context)
    external fun onShareUrl(url: String)

    @JvmStatic var activity: Activity? = null

    @JvmStatic
    fun shareFile(path: String) {
        val act = activity ?: return
        val file = java.io.File(path)
        if (!file.exists() || !file.canRead()) return
        val uri = FileProvider.getUriForFile(act, "${act.packageName}.fileprovider", file)
        val sendIntent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        act.startActivity(Intent.createChooser(sendIntent, null))
    }
}
