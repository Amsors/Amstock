package com.amsors.amstock_app

import java.net.URI

object NavigationPolicy {
    const val HOME_URL = "https://amstock.amsors.top/"

    private const val APP_HOST = "amstock.amsors.top"
    private const val CLOUDFLARE_ACCESS_HOST = "cloudflareaccess.com"

    fun shouldOpenInsideApp(rawUrl: String): Boolean {
        if (rawUrl == "about:blank") return true
        val uri = parse(rawUrl) ?: return false
        if (!uri.scheme.equals("https", ignoreCase = true)) return false
        val host = uri.host?.lowercase() ?: return false
        return host == APP_HOST ||
            host == CLOUDFLARE_ACCESS_HOST ||
            host.endsWith(".$CLOUDFLARE_ACCESS_HOST")
    }

    fun isSecureHttpUrl(rawUrl: String): Boolean =
        parse(rawUrl)?.scheme.equals("https", ignoreCase = true)

    private fun parse(rawUrl: String): URI? = try {
        URI(rawUrl)
    } catch (_: Exception) {
        null
    }
}
