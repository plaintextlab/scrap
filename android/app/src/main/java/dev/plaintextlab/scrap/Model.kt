package dev.plaintextlab.scrap

import org.json.JSONArray

data class NoteRef(val rel: String, val name: String, val modified: Long = 0) {
    val folder: String get() = parentOf(rel)
}

data class SearchHit(val rel: String, val name: String, val text: String)

data class OutLink(val target: String, val exists: Boolean)

sealed interface Screen {
    data class Browse(val folder: String) : Screen
    data class Note(val rel: String) : Screen
    data object Settings : Screen
    data object SyncSetup : Screen
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
