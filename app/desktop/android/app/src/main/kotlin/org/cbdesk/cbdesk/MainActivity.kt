package org.cbdesk.cbdesk

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.media.projection.MediaProjectionManager
import android.os.Build
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/// Flutter host. Screen pixels are read only after the system capture dialog
/// returns RESULT_OK. A denial does not create a virtual display.
class MainActivity : FlutterActivity() {
    private var pendingStart: MethodChannel.Result? = null
    private var pendingMic: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        val methodChannel = MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            "org.cbdesk/capture",
        )
        CaptureBridge.channel = methodChannel
        methodChannel.setMethodCallHandler { call, result ->
            when (call.method) {
                "filesDir" -> result.success(filesDir.absolutePath)
                "start" -> beginShare(result)
                "stop" -> {
                    CaptureService.stop(this)
                    result.success(null)
                }
                "requestMic" -> requestMic(result)
                else -> result.notImplemented()
            }
        }
    }

    override fun onDestroy() {
        if (pendingStart != null) {
            pendingStart?.success(false)
            pendingStart = null
        }
        CaptureBridge.channel = null
        CaptureBridge.onReady = null
        CaptureService.stop(this)
        super.onDestroy()
    }

    @Deprecated("Activity result API. FlutterActivity is not a ComponentActivity.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQ_CAPTURE) {
            return
        }
        val pending = pendingStart
        if (resultCode != Activity.RESULT_OK || data == null) {
            pending?.success(false)
            pendingStart = null
            return
        }
        CaptureBridge.onReady = { ok ->
            pending?.success(ok)
            pendingStart = null
            CaptureBridge.onReady = null
        }
        CaptureService.start(this, resultCode, data)
    }

    @Deprecated("Permission result API. FlutterActivity is not a ComponentActivity.")
    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        @Suppress("DEPRECATION")
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        val granted = grantResults.isNotEmpty() &&
            grantResults[0] == PackageManager.PERMISSION_GRANTED
        when (requestCode) {
            REQ_NOTIF -> {
                if (!granted) {
                    pendingStart?.success(false)
                    pendingStart = null
                } else {
                    launchCapture()
                }
            }
            REQ_MIC -> {
                pendingMic?.success(granted)
                pendingMic = null
            }
        }
    }

    private fun beginShare(result: MethodChannel.Result) {
        if (pendingStart != null) {
            result.error("busy", "capture consent is already open", null)
            return
        }
        pendingStart = result
        if (Build.VERSION.SDK_INT >= 33 && !hasPermission(Manifest.permission.POST_NOTIFICATIONS)) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQ_NOTIF)
            return
        }
        launchCapture()
    }

    private fun launchCapture() {
        val manager = getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        @Suppress("DEPRECATION")
        startActivityForResult(manager.createScreenCaptureIntent(), REQ_CAPTURE)
    }

    private fun requestMic(result: MethodChannel.Result) {
        if (hasPermission(Manifest.permission.RECORD_AUDIO)) {
            result.success(true)
            return
        }
        if (pendingMic != null) {
            result.error("busy", "microphone consent is already open", null)
            return
        }
        pendingMic = result
        requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), REQ_MIC)
    }

    private fun hasPermission(name: String): Boolean {
        return checkSelfPermission(name) == PackageManager.PERMISSION_GRANTED
    }

    companion object {
        private const val REQ_CAPTURE = 22116
        private const val REQ_NOTIF = 22117
        private const val REQ_MIC = 22118
    }
}
