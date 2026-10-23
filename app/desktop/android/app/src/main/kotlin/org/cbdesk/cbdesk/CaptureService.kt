package org.cbdesk.cbdesk

import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.app.Service
import android.os.IBinder
import io.flutter.plugin.common.MethodChannel
import kotlin.math.max

/**
 * Screen capture after the system consent dialog.
 *
 * The virtual display is created only after [MediaProjectionManager.getMediaProjection]
 * returns, and that call happens only after the person accepts the system dialog.
 */
object CaptureBridge {
    var channel: MethodChannel? = null
    var onReady: ((Boolean) -> Unit)? = null

    @Volatile
    var frameBusy: Boolean = false

    fun report(ok: Boolean) {
        val callback = onReady
        onReady = null
        Handler(Looper.getMainLooper()).post {
            callback?.invoke(ok)
        }
    }

    fun sendFrame(width: Int, height: Int, rgba: ByteArray) {
        val methodChannel = channel ?: return
        if (frameBusy) {
            return
        }
        frameBusy = true
        Handler(Looper.getMainLooper()).post {
            methodChannel.invokeMethod(
                "frame",
                mapOf("width" to width, "height" to height, "rgba" to rgba),
                object : MethodChannel.Result {
                    override fun success(result: Any?) {
                        frameBusy = false
                    }

                    override fun error(errorCode: String, errorMessage: String?, errorDetails: Any?) {
                        frameBusy = false
                    }

                    override fun notImplemented() {
                        frameBusy = false
                    }
                },
            )
        }
    }
}

class CaptureService : Service() {
    private var projection: MediaProjection? = null
    private var display: VirtualDisplay? = null
    private var reader: ImageReader? = null
    private val thread = HandlerThread("cbdesk-capture")
    private lateinit var handler: Handler

    override fun onCreate() {
        super.onCreate()
        thread.start()
        handler = Handler(thread.looper)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val code = intent?.getIntExtra(EXTRA_CODE, Activity.RESULT_CANCELED) ?: Activity.RESULT_CANCELED
        val data = readResult(intent)
        if (code != Activity.RESULT_OK || data == null) {
            reportAndStop(false)
            return START_NOT_STICKY
        }
        try {
            val notification = sessionNotification()
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    NOTIFICATION_ID,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION,
                )
            } else {
                startForeground(NOTIFICATION_ID, notification)
            }
            val manager = getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
            val created = manager.getMediaProjection(code, data)
            if (created == null) {
                reportAndStop(false)
                return START_NOT_STICKY
            }
            created.registerCallback(
                object : MediaProjection.Callback() {
                    override fun onStop() {
                        releaseCapture()
                    }
                },
                handler,
            )
            projection = created
            openDisplay(created)
            CaptureBridge.report(true)
        } catch (error: RuntimeException) {
            reportAndStop(false)
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        releaseCapture()
        thread.quitSafely()
        super.onDestroy()
    }

    private fun openDisplay(created: MediaProjection) {
        val (width, height) = captureSize()
        val dpi = resources.displayMetrics.densityDpi
        val images = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 2)
        images.setOnImageAvailableListener({ source -> copyFrame(source) }, handler)
        reader = images
        display = created.createVirtualDisplay(
            "cbdesk",
            width,
            height,
            dpi,
            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
            images.surface,
            null,
            handler,
        )
    }

    private fun copyFrame(source: ImageReader) {
        val image = source.acquireLatestImage() ?: return
        try {
            if (image.planes.isEmpty()) {
                return
            }
            val plane = image.planes[0]
            val buffer = plane.buffer
            val pixelStride = plane.pixelStride
            val rowStride = plane.rowStride
            val evenWidth = image.width - image.width % 2
            val evenHeight = image.height - image.height % 2
            if (evenWidth < 2 || evenHeight < 2 || pixelStride < 4) {
                return
            }
            val packed = ByteArray(evenWidth * evenHeight * 4)
            var out = 0
            for (y in 0 until evenHeight) {
                for (x in 0 until evenWidth) {
                    val index = y * rowStride + x * pixelStride
                    if (index + 3 >= buffer.capacity()) {
                        return
                    }
                    packed[out++] = buffer.get(index)
                    packed[out++] = buffer.get(index + 1)
                    packed[out++] = buffer.get(index + 2)
                    packed[out++] = buffer.get(index + 3)
                }
            }
            CaptureBridge.sendFrame(evenWidth, evenHeight, packed)
        } finally {
            image.close()
        }
    }

    private fun captureSize(): Pair<Int, Int> {
        val metrics = resources.displayMetrics
        var width = metrics.widthPixels
        var height = metrics.heightPixels
        val longSide = max(width, height)
        if (longSide > MAX_SIDE) {
            val scale = MAX_SIDE.toFloat() / longSide.toFloat()
            width = (width * scale).toInt()
            height = (height * scale).toInt()
        }
        if (width % 2 != 0) {
            width -= 1
        }
        if (height % 2 != 0) {
            height -= 1
        }
        if (width < 2) {
            width = 2
        }
        if (height < 2) {
            height = 2
        }
        return width to height
    }

    private fun sessionNotification(): Notification {
        val channelId = "cbdesk-session"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = getSystemService(NotificationManager::class.java)
            val channel = NotificationChannel(
                channelId,
                "CBDesk session",
                NotificationManager.IMPORTANCE_LOW,
            )
            channel.description = "Shown while this screen is being shared"
            manager.createNotificationChannel(channel)
        }
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, channelId)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("CBDesk")
            .setContentText("This screen is being shared")
            .setSmallIcon(android.R.drawable.ic_menu_view)
            .setOngoing(true)
            .build()
    }

    private fun reportAndStop(ok: Boolean) {
        CaptureBridge.report(ok)
        stopSelf()
    }

    private fun releaseCapture() {
        display?.release()
        display = null
        reader?.close()
        reader = null
        projection?.stop()
        projection = null
    }

    companion object {
        const val EXTRA_CODE = "resultCode"
        const val EXTRA_DATA = "resultData"
        private const val NOTIFICATION_ID = 22116
        private const val MAX_SIDE = 960

        fun start(context: Context, resultCode: Int, data: Intent) {
            val intent = Intent(context, CaptureService::class.java)
            intent.putExtra(EXTRA_CODE, resultCode)
            intent.putExtra(EXTRA_DATA, data)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, CaptureService::class.java))
        }
    }
}

private fun readResult(intent: Intent?): Intent? {
    if (intent == null) {
        return null
    }
    return if (Build.VERSION.SDK_INT >= 33) {
        intent.getParcelableExtra(CaptureService.EXTRA_DATA, Intent::class.java)
    } else {
        @Suppress("DEPRECATION")
        intent.getParcelableExtra(CaptureService.EXTRA_DATA)
    }
}
