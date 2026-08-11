package com.amsors.amstock_app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ExampleUnitTest {
    @Test
    fun configuredHomeUrlStaysInsideWebView() {
        assertTrue(NavigationPolicy.shouldOpenInsideApp(NavigationPolicy.HOME_URL))
    }

    @Test
    fun appAndCloudflareAccessUrlsStayInsideWebView() {
        assertTrue(NavigationPolicy.shouldOpenInsideApp("https://amstock.amsors.top/"))
        assertTrue(
            NavigationPolicy.shouldOpenInsideApp(
                "https://example.cloudflareaccess.com/cdn-cgi/access/login",
            ),
        )
    }

    @Test
    fun insecureAndLookalikeHostsAreRejected() {
        assertFalse(NavigationPolicy.shouldOpenInsideApp("http://amstock.amsors.top/"))
        assertFalse(NavigationPolicy.shouldOpenInsideApp("https://amstock.amsors.top.example.com/"))
        assertFalse(NavigationPolicy.shouldOpenInsideApp("https://cloudflareaccess.com.example.com/"))
        assertFalse(NavigationPolicy.shouldOpenInsideApp("javascript:alert(1)"))
    }

    @Test
    fun scannedItemCodesAreNormalizedAndValidated() {
        assertEquals(
            "A-12-34-567890",
            NavigationPolicy.normalizeItemCode("  a-12-34-567890\n"),
        )
        assertNull(NavigationPolicy.normalizeItemCode("A-1-34-567890"))
        assertNull(NavigationPolicy.normalizeItemCode("https://example.com/"))
        assertNull(NavigationPolicy.normalizeItemCode(null))
    }

    @Test
    fun displayUrlUsesTheConfiguredAppOrigin() {
        val url = NavigationPolicy.displayUrlForCode("A-12-34-567890")
        assertTrue(url.endsWith("/display/A-12-34-567890"))
        assertTrue(NavigationPolicy.shouldOpenInsideApp(url))
    }
}
