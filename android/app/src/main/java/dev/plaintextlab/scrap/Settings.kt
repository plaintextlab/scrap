package dev.plaintextlab.scrap

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

enum class ThemeMode { System, Light, Dark }

enum class SortOrder { Name, Recent }

data class SyncSettings(
    val url: String = "",
    val username: String = "",
    val branch: String = "main",
    val authorName: String = "",
    val authorEmail: String = "",
    /** Sync when the app opens and when it goes to the background. */
    val auto: Boolean = true,
    /** Periodic sync via WorkManager while the app isn't open. */
    val background: Boolean = false,
    val hasToken: Boolean = false,
) {
    val configured: Boolean get() = url.isNotBlank() && hasToken
}

data class AppSettings(
    val theme: ThemeMode = ThemeMode.System,
    /** Multiplier for note text (editor and reader). */
    val textScale: Float = 1f,
    val openInEdit: Boolean = false,
    val sort: SortOrder = SortOrder.Name,
    val sync: SyncSettings = SyncSettings(),
)

data class SyncStatus(val time: Long, val message: String, val failed: Boolean)

/**
 * One set of notes: its own folder on the phone and its own GitHub repository.
 * [id] "default" is the original vault (folder `vault`); others live in `vaults/<id>`.
 */
data class Repo(
    val id: String,
    /** Optional display name; falls back to the repository's name. */
    val name: String = "",
    val url: String = "",
    val username: String = "",
    val branch: String = "main",
    val authorName: String = "",
    val authorEmail: String = "",
) {
    val label: String
        get() = name.ifBlank { repoName(url) }.ifBlank { "Notes on this phone" }

    companion object {
        const val DEFAULT = "default"

        /** "https://github.com/you/notes.git" -> "notes". */
        fun repoName(url: String): String = url.trimEnd('/').substringAfterLast('/').removeSuffix(".git")
    }
}

/** Settings in SharedPreferences; the access token is encrypted with an Android Keystore key. */
class SettingsStore(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    fun load(): AppSettings = with(prefs) {
        val repo = activeRepo()
        AppSettings(
            theme = enumOr(getString("theme", null), ThemeMode.System),
            textScale = getFloat("textScale", 1f),
            openInEdit = getBoolean("openInEdit", false),
            sort = enumOr(getString("sort", null), SortOrder.Name),
            sync = SyncSettings(
                url = repo.url,
                username = repo.username,
                branch = repo.branch,
                authorName = repo.authorName,
                authorEmail = repo.authorEmail,
                auto = getBoolean("sync.auto", true),
                background = getBoolean("sync.background", false),
                hasToken = contains(tokenKey(repo.id)),
            ),
        )
    }

    fun save(s: AppSettings) {
        prefs.edit()
            .putString("theme", s.theme.name)
            .putFloat("textScale", s.textScale)
            .putBoolean("openInEdit", s.openInEdit)
            .putString("sort", s.sort.name)
            .putBoolean("sync.auto", s.sync.auto)
            .putBoolean("sync.background", s.sync.background)
            .apply()
        saveRepo(
            activeRepo().copy(
                url = s.sync.url,
                username = s.sync.username,
                branch = s.sync.branch,
                authorName = s.sync.authorName,
                authorEmail = s.sync.authorEmail,
            ),
        )
    }

    // ---- Repositories -----------------------------------------------------

    fun repos(): List<Repo> {
        val json = prefs.getString("repos", null)
            ?: return listOf(
                // Before multiple repositories: the single connection becomes the first one.
                Repo(
                    id = Repo.DEFAULT,
                    url = prefs.getString("sync.url", "")!!,
                    username = prefs.getString("sync.username", "")!!,
                    branch = prefs.getString("sync.branch", "main")!!,
                    authorName = prefs.getString("sync.name", "")!!,
                    authorEmail = prefs.getString("sync.email", "")!!,
                ),
            )
        val a = JSONArray(json)
        return List(a.length()) { i ->
            val o = a.getJSONObject(i)
            Repo(
                id = o.getString("id"),
                name = o.optString("name"),
                url = o.optString("url"),
                username = o.optString("username"),
                branch = o.optString("branch", "main").ifBlank { "main" },
                authorName = o.optString("authorName"),
                authorEmail = o.optString("authorEmail"),
            )
        }
    }

    private fun writeRepos(list: List<Repo>) {
        val a = JSONArray()
        for (r in list) {
            a.put(
                JSONObject()
                    .put("id", r.id)
                    .put("name", r.name)
                    .put("url", r.url)
                    .put("username", r.username)
                    .put("branch", r.branch)
                    .put("authorName", r.authorName)
                    .put("authorEmail", r.authorEmail),
            )
        }
        prefs.edit().putString("repos", a.toString()).apply()
    }

    /** Adds or updates a repository (matched by id). */
    fun saveRepo(repo: Repo) {
        val list = repos()
        writeRepos(if (list.any { it.id == repo.id }) list.map { if (it.id == repo.id) repo else it } else list + repo)
    }

    fun removeRepo(id: String) {
        writeRepos(repos().filter { it.id != id })
        clearSyncStatus(id)
        prefs.edit().remove(tokenKey(id)).apply()
    }

    fun activeRepoId(): String = prefs.getString("activeRepo", Repo.DEFAULT)!!

    fun activeRepo(): Repo = repos().let { list -> list.firstOrNull { it.id == activeRepoId() } ?: list.first() }

    fun setActiveRepo(id: String) {
        prefs.edit().putString("activeRepo", id).apply()
    }

    // Per-repository keys; the original repository keeps the keys it had before.
    private fun tokenKey(id: String) = if (id == Repo.DEFAULT) "sync.token" else "sync.token.$id"

    private fun statusKey(base: String, id: String) = if (id == Repo.DEFAULT) base else "$base.$id"

    fun hasToken(id: String): Boolean = prefs.contains(tokenKey(id))

    fun token(id: String = activeRepoId()): String? = prefs.getString(tokenKey(id), null)?.let(TokenCipher::decrypt)

    fun setToken(token: String?, id: String = activeRepoId()) {
        prefs.edit().apply {
            if (token.isNullOrBlank()) remove(tokenKey(id)) else putString(tokenKey(id), TokenCipher.encrypt(token))
        }.apply()
    }

    fun lastSync(id: String = activeRepoId()): SyncStatus? {
        val time = prefs.getLong(statusKey("sync.lastTime", id), 0)
        if (time == 0L) return null
        return SyncStatus(
            time,
            prefs.getString(statusKey("sync.lastMessage", id), "")!!,
            prefs.getBoolean(statusKey("sync.lastFailed", id), false),
        )
    }

    fun recordSync(message: String, failed: Boolean, id: String = activeRepoId()) {
        prefs.edit()
            .putLong(statusKey("sync.lastTime", id), System.currentTimeMillis())
            .putString(statusKey("sync.lastMessage", id), message)
            .putBoolean(statusKey("sync.lastFailed", id), failed)
            .apply()
    }

    fun clearSyncStatus(id: String = activeRepoId()) {
        prefs.edit()
            .remove(statusKey("sync.lastTime", id))
            .remove(statusKey("sync.lastMessage", id))
            .remove(statusKey("sync.lastFailed", id))
            .apply()
    }

    private inline fun <reified E : Enum<E>> enumOr(name: String?, default: E): E =
        enumValues<E>().firstOrNull { it.name == name } ?: default
}

/** AES-GCM with a non-exportable key held by the Android Keystore. */
private object TokenCipher {
    private const val ALIAS = "scrap-sync-token"
    private const val TRANSFORM = "AES/GCM/NoPadding"
    private const val IV_BYTES = 12

    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build(),
        )
        return generator.generateKey()
    }

    fun encrypt(plain: String): String {
        val cipher = Cipher.getInstance(TRANSFORM).apply { init(Cipher.ENCRYPT_MODE, key()) }
        return Base64.encodeToString(cipher.iv + cipher.doFinal(plain.toByteArray()), Base64.NO_WRAP)
    }

    /** Null if the data can't be decrypted (e.g. restored onto another device). */
    fun decrypt(encoded: String): String? = runCatching {
        val bytes = Base64.decode(encoded, Base64.NO_WRAP)
        val cipher = Cipher.getInstance(TRANSFORM)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes, 0, IV_BYTES))
        String(cipher.doFinal(bytes, IV_BYTES, bytes.size - IV_BYTES))
    }.getOrNull()
}
