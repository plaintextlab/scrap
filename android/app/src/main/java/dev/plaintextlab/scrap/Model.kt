package dev.plaintextlab.scrap

import org.json.JSONArray

data class NoteRef(val rel: String, val name: String) {
    val folder: String get() = parentOf(rel)
}

data class SearchHit(val rel: String, val name: String, val text: String)

data class OutLink(val target: String, val exists: Boolean)

sealed interface Screen {
    data class Browse(val folder: String) : Screen
    data class Note(val rel: String) : Screen
    data object Search : Screen
}

fun parentOf(rel: String): String = rel.substringBeforeLast('/', "")

fun nameOf(rel: String): String = rel.substringAfterLast('/')

fun titleOf(rel: String): String = nameOf(rel).removeSuffix(".md")

internal inline fun <T> JSONArray.map(f: (Int) -> T): List<T> = List(length()) { f(it) }
