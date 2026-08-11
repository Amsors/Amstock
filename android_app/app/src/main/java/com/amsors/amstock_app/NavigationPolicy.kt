package com.amsors.amstock_app

import java.net.URI

object NavigationPolicy {
    val HOME_URL: String = BuildConfig.HOME_URL

    private const val APP_HOST = "amstock.amsors.top"
    private const val CLOUDFLARE_ACCESS_HOST = "cloudflareaccess.com"
    private val configuredHome = parse(HOME_URL)

    fun shouldOpenInsideApp(rawUrl: String): Boolean {
        if (rawUrl == "about:blank") return true
        val uri = parse(rawUrl) ?: return false
        if (configuredHome != null && hasSameOrigin(uri, configuredHome)) return true
        if (!uri.scheme.equals("https", ignoreCase = true)) return false
        val host = uri.host?.lowercase() ?: return false
        return host == APP_HOST ||
            host == CLOUDFLARE_ACCESS_HOST ||
            host.endsWith(".$CLOUDFLARE_ACCESS_HOST")
    }

    fun isSecureHttpUrl(rawUrl: String): Boolean =
        parse(rawUrl)?.scheme.equals("https", ignoreCase = true)

    private fun hasSameOrigin(left: URI, right: URI): Boolean =
        left.scheme.equals(right.scheme, ignoreCase = true) &&
            left.host.equals(right.host, ignoreCase = true) &&
            effectivePort(left) == effectivePort(right)

    private fun effectivePort(uri: URI): Int = when {
        uri.port >= 0 -> uri.port
        uri.scheme.equals("https", ignoreCase = true) -> 443
        uri.scheme.equals("http", ignoreCase = true) -> 80
        else -> -1
    }

    private fun parse(rawUrl: String): URI? = try {
        URI(rawUrl)
    } catch (_: Exception) {
        null
    }
}
