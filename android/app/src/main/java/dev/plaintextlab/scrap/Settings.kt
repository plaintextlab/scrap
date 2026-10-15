package dev.plaintextlab.scrap

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
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

/** Settings in SharedPreferences; the access token is encrypted with an Android Keystore key. */
class SettingsStore(context: Context) {
    private val prefs = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    fun load(): AppSettings = with(prefs) {
        AppSettings(
            theme = enumOr(getString("theme", null), ThemeMode.System),
            textScale = getFloat("textScale", 1f),
            openInEdit = getBoolean("openInEdit", false),
            sort = enumOr(getString("sort", null), SortOrder.Name),
            sync = SyncSettings(
                url = getString("sync.url", "")!!,
                username = getString("sync.username", "")!!,
                branch = getString("sync.branch", "main")!!,
                authorName = getString("sync.name", "")!!,
                authorEmail = getString("sync.email", "")!!,
                auto = getBoolean("sync.auto", true),
                background = getBoolean("sync.background", false),
                hasToken = contains("sync.token"),
            ),
        )
    }

    fun save(s: AppSettings) {
        prefs.edit()
            .putString("theme", s.theme.name)
            .putFloat("textScale", s.textScale)
            .putBoolean("openInEdit", s.openInEdit)
            .putString("sort", s.sort.name)
            .putString("sync.url", s.sync.url)
            .putString("sync.username", s.sync.username)
            .putString("sync.branch", s.sync.branch)
            .putString("sync.name", s.sync.authorName)
            .putString("sync.email", s.sync.authorEmail)
            .putBoolean("sync.auto", s.sync.auto)
            .putBoolean("sync.background", s.sync.background)
            .apply()
    }

    fun token(): String? = prefs.getString("sync.token", null)?.let(TokenCipher::decrypt)

    fun setToken(token: String?) {
        prefs.edit().apply {
            if (token.isNullOrBlank()) remove("sync.token") else putString("sync.token", TokenCipher.encrypt(token))
        }.apply()
    }

    fun lastSync(): SyncStatus? {
        val time = prefs.getLong("sync.lastTime", 0)
        if (time == 0L) return null
        return SyncStatus(time, prefs.getString("sync.lastMessage", "")!!, prefs.getBoolean("sync.lastFailed", false))
    }

    fun recordSync(message: String, failed: Boolean) {
        prefs.edit()
            .putLong("sync.lastTime", System.currentTimeMillis())
            .putString("sync.lastMessage", message)
            .putBoolean("sync.lastFailed", failed)
            .apply()
    }

    fun clearSyncStatus() {
        prefs.edit().remove("sync.lastTime").remove("sync.lastMessage").remove("sync.lastFailed").apply()
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
