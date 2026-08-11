package com.amsors.amstock_app

import android.annotation.SuppressLint
import android.app.DownloadManager
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.Uri
import android.os.Bundle
import android.os.Environment
import android.provider.MediaStore
import android.view.View
import android.webkit.CookieManager
import android.webkit.DownloadListener
import android.webkit.URLUtil
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.enableEdgeToEdge
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.exifinterface.media.ExifInterface
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.concurrent.Executors
import kotlin.math.sqrt

class MainActivity : AppCompatActivity() {
    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var errorPanel: View
    private lateinit var errorMessage: TextView
    private lateinit var photoProcessingPanel: View

    private var fileChooserCallback: ValueCallback<Array<Uri>>? = null
    private var pendingCameraFile: File? = null
    private var pendingCameraUri: Uri? = null
    private val imageExecutor = Executors.newSingleThreadExecutor()

    private val fileChooserLauncher =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val callback = fileChooserCallback ?: return@registerForActivityResult
            if (result.resultCode != RESULT_OK) {
                pendingCameraFile?.delete()
                callback.onReceiveValue(null)
                clearPendingFileRequest()
                return@registerForActivityResult
            }

            val capturedUris = capturedPhotoUris()
            if (capturedUris.isNotEmpty()) {
                compressAndReturnCameraPhoto(callback, capturedUris.first())
            } else {
                val selectedUris = extractSelectedUris(result.data)
                    .takeIf { it.isNotEmpty() }
                    ?.toTypedArray()
                pendingCameraFile?.delete()
                callback.onReceiveValue(selectedUris)
                clearPendingFileRequest()
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContentView(R.layout.activity_main)

        ViewCompat.setOnApplyWindowInsetsListener(findViewById(R.id.main)) { v, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
            )
            v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }

        webView = findViewById(R.id.web_view)
        progressBar = findViewById(R.id.page_progress)
        errorPanel = findViewById(R.id.error_panel)
        errorMessage = findViewById(R.id.error_message)
        photoProcessingPanel = findViewById(R.id.photo_processing_panel)
        findViewById<Button>(R.id.retry_button).setOnClickListener { loadHomePage() }
        findViewById<MaterialToolbar>(R.id.app_toolbar).setOnMenuItemClickListener { item ->
            if (item.itemId == R.id.action_image_settings) {
                showImageSelectionSettings()
                true
            } else {
                false
            }
        }

        configureWebView()
        configureBackNavigation()

        if (savedInstanceState == null || webView.restoreState(savedInstanceState) == null) {
            loadHomePage()
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        webView.saveState(outState)
        super.onSaveInstanceState(outState)
    }

    override fun onPause() {
        CookieManager.getInstance().flush()
        webView.onPause()
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        webView.onResume()
    }

    override fun onDestroy() {
        fileChooserCallback?.onReceiveValue(null)
        fileChooserCallback = null
        imageExecutor.shutdownNow()
        webView.stopLoading()
        webView.webChromeClient = null
        webView.webViewClient = WebViewClient()
        webView.destroy()
        super.onDestroy()
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun configureWebView() {
        WebView.setWebContentsDebuggingEnabled(
            (applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE) != 0,
        )

        CookieManager.getInstance().apply {
            setAcceptCookie(true)
            setAcceptThirdPartyCookies(webView, true)
        }

        webView.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = true
            mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            cacheMode = WebSettings.LOAD_DEFAULT
            builtInZoomControls = false
            displayZoomControls = false
            setSupportMultipleWindows(false)
            mediaPlaybackRequiresUserGesture = true
            userAgentString = "$userAgentString AmstockAndroid/${BuildConfig.VERSION_NAME}"
        }

        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                val uri = request.url
                if (NavigationPolicy.shouldOpenInsideApp(uri.toString())) return false
                openExternalUri(uri)
                return true
            }

            override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
                progressBar.visibility = View.VISIBLE
                errorPanel.visibility = View.GONE
            }

            override fun onPageFinished(view: WebView, url: String) {
                progressBar.visibility = View.GONE
                CookieManager.getInstance().flush()
            }

            override fun onReceivedError(
                view: WebView,
                request: WebResourceRequest,
                error: WebResourceError,
            ) {
                if (request.isForMainFrame) {
                    showLoadError(error.description?.toString())
                }
            }
        }

        webView.webChromeClient = object : WebChromeClient() {
            override fun onProgressChanged(view: WebView, newProgress: Int) {
                progressBar.progress = newProgress
                progressBar.visibility = if (newProgress < 100) View.VISIBLE else View.GONE
            }

            override fun onShowFileChooser(
                webView: WebView,
                filePathCallback: ValueCallback<Array<Uri>>,
                fileChooserParams: FileChooserParams,
            ): Boolean {
                fileChooserCallback?.onReceiveValue(null)
                pendingCameraFile?.delete()
                clearPendingFileRequest()
                fileChooserCallback = filePathCallback

                return try {
                    fileChooserLauncher.launch(createFileChooserIntent(fileChooserParams))
                    true
                } catch (_: Exception) {
                    Toast.makeText(
                        this@MainActivity,
                        R.string.no_file_picker,
                        Toast.LENGTH_SHORT,
                    ).show()
                    fileChooserCallback?.onReceiveValue(null)
                    pendingCameraFile?.delete()
                    clearPendingFileRequest()
                    true
                }
            }
        }

        webView.setDownloadListener(createDownloadListener())
    }

    private fun configureBackNavigation() {
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (errorPanel.visibility == View.VISIBLE) {
                    loadHomePage()
                } else if (webView.canGoBack()) {
                    webView.goBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                }
            }
        })
    }

    private fun loadHomePage() {
        errorPanel.visibility = View.GONE
        if (!isNetworkAvailable()) {
            showLoadError(getString(R.string.network_unavailable))
            return
        }
        webView.loadUrl(NavigationPolicy.HOME_URL)
    }

    private fun showLoadError(detail: String?) {
        webView.stopLoading()
        progressBar.visibility = View.GONE
        errorMessage.text = detail
            ?.takeIf { it.isNotBlank() }
            ?.let { getString(R.string.load_failed_with_reason, it) }
            ?: getString(R.string.load_failed)
        errorPanel.visibility = View.VISIBLE
    }

    private fun createFileChooserIntent(params: WebChromeClient.FileChooserParams): Intent {
        val acceptedTypes = params.acceptTypes
            .flatMap { it.split(',') }
            .map(String::trim)
            .filter(String::isNotEmpty)
            .distinct()

        val picker = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = when {
                acceptedTypes.isEmpty() -> "*/*"
                acceptedTypes.size == 1 -> acceptedTypes.first()
                else -> "*/*"
            }
            if (acceptedTypes.size > 1) {
                putExtra(Intent.EXTRA_MIME_TYPES, acceptedTypes.toTypedArray())
            }
            putExtra(
                Intent.EXTRA_ALLOW_MULTIPLE,
                params.mode == WebChromeClient.FileChooserParams.MODE_OPEN_MULTIPLE,
            )
        }

        val acceptsImages = acceptedTypes.isEmpty() || acceptedTypes.any {
            it == "*/*" || it.startsWith("image/") || it.startsWith(".jpg") ||
                it.startsWith(".jpeg") || it.startsWith(".png") || it.startsWith(".webp")
        }
        if (!acceptsImages) return picker

        val selectionMode = getImageSelectionMode()
        val camera = if (selectionMode != ImageSelectionMode.FILES) createCameraIntent() else null
        return when (selectionMode) {
            ImageSelectionMode.CAMERA -> camera ?: picker.also {
                Toast.makeText(this, R.string.camera_unavailable, Toast.LENGTH_SHORT).show()
            }

            ImageSelectionMode.FILES -> picker

            ImageSelectionMode.ASK -> Intent.createChooser(
                picker,
                getString(R.string.choose_image),
            ).apply {
                camera?.let { putExtra(Intent.EXTRA_INITIAL_INTENTS, arrayOf(it)) }
            }
        }
    }

    private fun createCameraIntent(): Intent? {
        val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE)
        if (intent.resolveActivity(packageManager) == null) return null

        val cameraDirectory = File(externalCacheDir ?: cacheDir, "camera").apply { mkdirs() }
        val photoFile = File.createTempFile("amstock_", ".jpg", cameraDirectory)
        val photoUri = FileProvider.getUriForFile(
            this,
            "${applicationContext.packageName}.fileprovider",
            photoFile,
        )
        pendingCameraFile = photoFile
        pendingCameraUri = photoUri

        return intent.apply {
            putExtra(MediaStore.EXTRA_OUTPUT, photoUri)
            clipData = ClipData.newRawUri(getString(R.string.camera_photo), photoUri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        }
    }

    private fun extractSelectedUris(data: Intent?): List<Uri> {
        val clipData = data?.clipData
        if (clipData != null) {
            return (0 until clipData.itemCount).map { clipData.getItemAt(it).uri }
        }
        return listOfNotNull(data?.data)
    }

    private fun capturedPhotoUris(): List<Uri> {
        val file = pendingCameraFile
        val uri = pendingCameraUri
        return if (file != null && file.exists() && file.length() > 0L && uri != null) {
            listOf(uri)
        } else {
            emptyList()
        }
    }

    private fun clearPendingFileRequest() {
        fileChooserCallback = null
        pendingCameraFile = null
        pendingCameraUri = null
    }

    private fun showImageSelectionSettings() {
        val modes = ImageSelectionMode.entries.toTypedArray()
        val labels = modes.map { getString(it.labelRes) }.toTypedArray()
        val current = modes.indexOf(getImageSelectionMode())

        MaterialAlertDialogBuilder(this)
            .setTitle(R.string.image_selection_setting_title)
            .setSingleChoiceItems(labels, current) { dialog, which ->
                getSharedPreferences(PREFERENCES_NAME, MODE_PRIVATE)
                    .edit()
                    .putString(IMAGE_SELECTION_MODE_KEY, modes[which].value)
                    .apply()
                Toast.makeText(
                    this,
                    getString(R.string.image_selection_saved, labels[which]),
                    Toast.LENGTH_SHORT,
                ).show()
                dialog.dismiss()
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun getImageSelectionMode(): ImageSelectionMode {
        val storedValue = getSharedPreferences(PREFERENCES_NAME, MODE_PRIVATE)
            .getString(IMAGE_SELECTION_MODE_KEY, null)
        return ImageSelectionMode.fromValue(storedValue)
    }

    private fun compressAndReturnCameraPhoto(
        callback: ValueCallback<Array<Uri>>,
        photoUri: Uri,
    ) {
        val photoFile = pendingCameraFile
        if (photoFile == null) {
            callback.onReceiveValue(arrayOf(photoUri))
            clearPendingFileRequest()
            return
        }

        photoProcessingPanel.visibility = View.VISIBLE
        imageExecutor.execute {
            val compressed = try {
                compressJpegToUploadLimit(photoFile)
            } catch (_: Exception) {
                false
            } catch (_: OutOfMemoryError) {
                false
            }

            runOnUiThread {
                if (isDestroyed) return@runOnUiThread
                photoProcessingPanel.visibility = View.GONE
                if (compressed && photoFile.length() in 1..MAX_UPLOAD_IMAGE_BYTES) {
                    callback.onReceiveValue(arrayOf(photoUri))
                } else {
                    photoFile.delete()
                    callback.onReceiveValue(null)
                    Toast.makeText(
                        this,
                        R.string.photo_compression_failed,
                        Toast.LENGTH_LONG,
                    ).show()
                }
                clearPendingFileRequest()
            }
        }
    }

    private fun compressJpegToUploadLimit(photoFile: File): Boolean {
        if (photoFile.length() in 1..MAX_UPLOAD_IMAGE_BYTES) return true

        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(photoFile.absolutePath, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return false

        var sampleSize = 1
        while (
            bounds.outWidth / sampleSize > MAX_DECODE_DIMENSION ||
            bounds.outHeight / sampleSize > MAX_DECODE_DIMENSION
        ) {
            sampleSize *= 2
        }

        val decoded = BitmapFactory.decodeFile(
            photoFile.absolutePath,
            BitmapFactory.Options().apply {
                inSampleSize = sampleSize
                inPreferredConfig = Bitmap.Config.ARGB_8888
            },
        ) ?: return false

        var working = rotateFromExif(decoded, photoFile)
        if (working !== decoded) decoded.recycle()

        var quality = 90
        var encoded = encodeJpeg(working, quality)
        var attempts = 0
        while (encoded.size > MAX_UPLOAD_IMAGE_BYTES && attempts < MAX_COMPRESSION_ATTEMPTS) {
            if (quality > MIN_JPEG_QUALITY) {
                quality = (quality - 8).coerceAtLeast(MIN_JPEG_QUALITY)
            } else {
                val scale = (sqrt(MAX_UPLOAD_IMAGE_BYTES.toDouble() / encoded.size) * 0.92)
                    .coerceIn(0.65, 0.9)
                val newWidth = (working.width * scale).toInt().coerceAtLeast(MIN_IMAGE_DIMENSION)
                val newHeight = (working.height * scale).toInt().coerceAtLeast(MIN_IMAGE_DIMENSION)
                val scaled = Bitmap.createScaledBitmap(working, newWidth, newHeight, true)
                if (scaled !== working) working.recycle()
                working = scaled
                quality = 82
            }
            encoded = encodeJpeg(working, quality)
            attempts++
        }
        working.recycle()

        if (encoded.size > MAX_UPLOAD_IMAGE_BYTES) return false
        photoFile.outputStream().use { it.write(encoded) }
        return photoFile.length() in 1..MAX_UPLOAD_IMAGE_BYTES
    }

    private fun rotateFromExif(bitmap: Bitmap, photoFile: File): Bitmap {
        val orientation = try {
            ExifInterface(photoFile.absolutePath).getAttributeInt(
                ExifInterface.TAG_ORIENTATION,
                ExifInterface.ORIENTATION_NORMAL,
            )
        } catch (_: Exception) {
            ExifInterface.ORIENTATION_NORMAL
        }

        val matrix = Matrix()
        when (orientation) {
            ExifInterface.ORIENTATION_ROTATE_90 -> matrix.postRotate(90f)
            ExifInterface.ORIENTATION_ROTATE_180 -> matrix.postRotate(180f)
            ExifInterface.ORIENTATION_ROTATE_270 -> matrix.postRotate(270f)
            ExifInterface.ORIENTATION_FLIP_HORIZONTAL -> matrix.preScale(-1f, 1f)
            ExifInterface.ORIENTATION_FLIP_VERTICAL -> matrix.preScale(1f, -1f)
            ExifInterface.ORIENTATION_TRANSPOSE -> {
                matrix.preScale(-1f, 1f)
                matrix.postRotate(270f)
            }

            ExifInterface.ORIENTATION_TRANSVERSE -> {
                matrix.preScale(-1f, 1f)
                matrix.postRotate(90f)
            }

            else -> return bitmap
        }
        return Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true)
    }

    private fun encodeJpeg(bitmap: Bitmap, quality: Int): ByteArray =
        ByteArrayOutputStream().use { output ->
            bitmap.compress(Bitmap.CompressFormat.JPEG, quality, output)
            output.toByteArray()
        }

    private fun openExternalUri(uri: Uri) {
        if (uri.scheme !in setOf("http", "https", "mailto", "tel")) {
            Toast.makeText(this, R.string.unsupported_link, Toast.LENGTH_SHORT).show()
            return
        }
        try {
            startActivity(Intent(Intent.ACTION_VIEW, uri))
        } catch (_: Exception) {
            Toast.makeText(this, R.string.no_app_for_link, Toast.LENGTH_SHORT).show()
        }
    }

    private fun createDownloadListener() = DownloadListener { url, userAgent, disposition, mimeType, _ ->
        if (!NavigationPolicy.isSecureHttpUrl(url)) {
            Toast.makeText(this, R.string.download_blocked, Toast.LENGTH_SHORT).show()
            return@DownloadListener
        }

        try {
            val fileName = URLUtil.guessFileName(url, disposition, mimeType)
            val request = DownloadManager.Request(Uri.parse(url)).apply {
                setMimeType(mimeType)
                addRequestHeader("User-Agent", userAgent)
                CookieManager.getInstance().getCookie(url)?.let { addRequestHeader("Cookie", it) }
                setTitle(fileName)
                setDescription(getString(R.string.downloading))
                setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                setDestinationInExternalFilesDir(
                    this@MainActivity,
                    Environment.DIRECTORY_DOWNLOADS,
                    fileName,
                )
            }
            (getSystemService(DOWNLOAD_SERVICE) as DownloadManager).enqueue(request)
            Toast.makeText(this, R.string.download_started, Toast.LENGTH_SHORT).show()
        } catch (_: Exception) {
            Toast.makeText(this, R.string.download_failed, Toast.LENGTH_SHORT).show()
        }
    }

    private fun isNetworkAvailable(): Boolean {
        val manager = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = manager.activeNetwork ?: return false
        val capabilities = manager.getNetworkCapabilities(network) ?: return false
        return capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
    }

    private enum class ImageSelectionMode(val value: String, val labelRes: Int) {
        CAMERA("camera", R.string.image_selection_camera),
        FILES("files", R.string.image_selection_files),
        ASK("ask", R.string.image_selection_ask);

        companion object {
            fun fromValue(value: String?): ImageSelectionMode =
                entries.firstOrNull { it.value == value } ?: ASK
        }
    }

    private companion object {
        const val PREFERENCES_NAME = "amstock_local_settings"
        const val IMAGE_SELECTION_MODE_KEY = "image_selection_mode"
        const val MAX_UPLOAD_IMAGE_BYTES = 2L * 1024L * 1024L
        const val MAX_DECODE_DIMENSION = 3072
        const val MIN_IMAGE_DIMENSION = 640
        const val MIN_JPEG_QUALITY = 58
        const val MAX_COMPRESSION_ATTEMPTS = 14
    }
}
