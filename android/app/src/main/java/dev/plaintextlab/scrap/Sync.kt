package dev.plaintextlab.scrap

import android.content.Context
import android.os.Build
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

/** The vault folder, opened at most once per process (by the UI or a background worker). */
object Vault {
    @Volatile private var opened = false

    fun dir(context: Context): File = File(context.getExternalFilesDir(null) ?: context.filesDir, "vault")

    @Synchronized
    fun ensureOpen(context: Context) {
        if (!opened) {
            Core.open(dir(context).path)
            opened = true
        }
    }
}

object Sync {
    /** Commits, pulls and pushes. Blocks; returns a summary or throws [IOException]. */
    suspend fun run(context: Context): String = withContext(Dispatchers.IO) {
        val store = SettingsStore(context)
        val s = store.load().sync
        val token = store.token()
        if (s.url.isBlank() || token == null) throw IOException("GitHub sync isn't set up")
        Vault.ensureOpen(context)
        val config = JSONObject()
            .put("url", s.url)
            .put("username", s.username)
            .put("token", token)
            .put("branch", s.branch)
            .put("name", s.authorName)
            .put("email", s.authorEmail)
            .put("device", Build.MODEL ?: "Android")
            .put("caFile", File(context.cacheDir, "cacerts.pem").path)
        try {
            Core.sync(config.toString()).also { store.recordSync(it, failed = false) }
        } catch (e: IOException) {
            store.recordSync(e.message ?: "Sync failed", failed = true)
            throw e
        }
    }

    private val online = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()

    /** Syncs as soon as there is a network, even if the app is closed meanwhile. */
    fun soon(context: Context) {
        val request = OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(online).build()
        WorkManager.getInstance(context).enqueueUniqueWork("sync-now", ExistingWorkPolicy.REPLACE, request)
    }

    fun schedulePeriodic(context: Context, enabled: Boolean) {
        val work = WorkManager.getInstance(context)
        if (enabled) {
            val request = PeriodicWorkRequestBuilder<SyncWorker>(1, TimeUnit.HOURS).setConstraints(online).build()
            work.enqueueUniquePeriodicWork("sync-periodic", ExistingPeriodicWorkPolicy.KEEP, request)
        } else {
            work.cancelUniqueWork("sync-periodic")
        }
    }
}

class SyncWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result = try {
        Sync.run(applicationContext)
        Result.success()
    } catch (e: Exception) {
        if (runAttemptCount < 2) Result.retry() else Result.failure()
    }
}
