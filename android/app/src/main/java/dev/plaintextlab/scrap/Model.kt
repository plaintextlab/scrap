package dev.plaintextlab.scrap

import org.json.JSONArray

data class NoteRef(
    val rel: String,
    val name: String,
    val modified: Long = 0,
    val pinned: Boolean = false,
    /** Background colour name, from the note's front matter. */
    val color: String? = null,
) {
    val folder: String get() = parentOf(rel)
}

/** Properties kept in a note's front matter (see core/src/meta.rs). */
data class NoteMeta(val pinned: Boolean, val color: String?, val tags: List<String>, val frontTags: List<String>) {
    companion object {
        fun of(text: String): NoteMeta {
            val o = org.json.JSONObject(Core.noteMeta(text))
            fun list(key: String) = o.getJSONArray(key).let { a -> (0 until a.length()).map { a.getString(it) } }
            return NoteMeta(
                pinned = o.optBoolean("pinned"),
                color = if (o.isNull("color")) null else o.getString("color"),
                tags = list("tags"),
                frontTags = list("frontTags"),
            )
        }
    }
}

data class SearchHit(val rel: String, val name: String, val text: String)

data class OutLink(val target: String, val exists: Boolean)

/** Something in the trash: [name] inside the trash, [original] where it was deleted from. */
data class TrashItem(val name: String, val original: String, val isDir: Boolean, val deleted: Long)

sealed interface Screen {
    data class Browse(val folder: String) : Screen
    data class Note(val rel: String) : Screen
    data object Trash : Screen
    data object Settings : Screen
    /** Connection details of the open repository, or a new one when [adding]. */
    data class SyncSetup(val adding: Boolean = false) : Screen
}

/** Accepts "you/notes", "github.com/you/notes" or a full HTTPS URL. Null for SSH URLs. */
fun normalizeRepoUrl(input: String): String? {
    val s = input.trim().removeSuffix("/")
    return when {
        s.isEmpty() || s.startsWith("git@") || s.startsWith("ssh://") -> null
        s.startsWith("https://") || s.startsWith("http://") -> s
        s.startsWith("github.com/") -> "https://$s"
        Regex("""^[\w.-]+/[\w.-]+$""").matches(s) -> "https://github.com/${s.removeSuffix(".git")}.git"
        else -> null
    }
}

fun parentOf(rel: String): String = rel.substringBeforeLast('/', "")

fun nameOf(rel: String): String = rel.substringAfterLast('/')

fun titleOf(rel: String): String = nameOf(rel).removeSuffix(".md")

internal inline fun <T> JSONArray.map(f: (Int) -> T): List<T> = List(length()) { f(it) }
