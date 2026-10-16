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

/**
 * The active repository's folder, opened by the Rust core. Shared by the UI and background
 * workers; switching repositories reopens it.
 */
object Vault {
    @Volatile private var openedPath: String? = null

    private fun base(context: Context): File = context.getExternalFilesDir(null) ?: context.filesDir

    /** Folder of a repository: the original keeps `vault`, the others get `vaults/<id>`. */
    fun dir(context: Context, id: String): File =
        if (id == Repo.DEFAULT) File(base(context), "vault") else File(base(context), "vaults/$id")

    fun dir(context: Context): File = dir(context, SettingsStore(context).activeRepoId())

    /** Opens the active repository's folder unless it's already the open one. */
    @Synchronized
    fun ensureOpen(context: Context) {
        val path = dir(context).path
        if (openedPath != path) {
            Core.open(path)
            openedPath = path
        }
    }
}

object Sync {
    /**
     * Commits, pulls and pushes a repository (the active one by default). Blocks; returns a
     * summary or throws [IOException].
     */
    suspend fun run(context: Context, repoId: String? = null): String = withContext(Dispatchers.IO) {
        val store = SettingsStore(context)
        val id = repoId ?: store.activeRepoId()
        val repo = store.repos().firstOrNull { it.id == id } ?: throw IOException("Unknown repository")
        val token = store.token(id)
        if (repo.url.isBlank() || token == null) throw IOException("GitHub sync isn't set up")
        val active = id == store.activeRepoId()
        if (active) Vault.ensureOpen(context)
        val config = JSONObject()
            .put("url", repo.url)
            .put("username", repo.username)
            .put("token", token)
            .put("branch", repo.branch)
            .put("name", repo.authorName)
            .put("email", repo.authorEmail)
            .put("device", Build.MODEL ?: "Android")
            // A repository that isn't open is synced in its own folder, leaving the open one alone.
            .put("root", if (active) "" else Vault.dir(context, id).path)
        try {
            Core.sync(config.toString()).also { store.recordSync(it, failed = false, id = id) }
        } catch (e: IOException) {
            store.recordSync(e.message ?: "Sync failed", failed = true, id = id)
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
