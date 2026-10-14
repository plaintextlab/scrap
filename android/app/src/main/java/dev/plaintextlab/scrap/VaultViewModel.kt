package dev.plaintextlab.scrap

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

class VaultViewModel(app: Application) : AndroidViewModel(app) {
    /** Navigation back stack; the last entry is on screen. */
    val stack = mutableStateListOf<Screen>(Screen.Browse(""))
    val screen: Screen get() = stack.last()

    var ready by mutableStateOf(false)
        private set
    var folders by mutableStateOf(emptyList<String>())
        private set
    var notes by mutableStateOf(emptyList<NoteRef>())
        private set
    var error by mutableStateOf<String?>(null)

    // The open note.
    var noteRel by mutableStateOf<String?>(null)
        private set
    var noteText by mutableStateOf("")
        private set
    var noteLoaded by mutableStateOf(false)
        private set
    var editing by mutableStateOf(false)
    private var savedText = ""
    private var saveJob: Job? = null

    init {
        val base = app.getExternalFilesDir(null) ?: app.filesDir
        launch {
            io { Core.open(File(base, "vault").path) }
            reload()
            ready = true
        }
    }

    private fun launch(block: suspend CoroutineScope.() -> Unit) = viewModelScope.launch {
        try {
            block()
        } catch (e: Exception) {
            error = e.message ?: "Something went wrong"
        }
    }

    private suspend fun <T> io(block: () -> T): T = withContext(Dispatchers.IO) { block() }

    private suspend fun reload() {
        val json = JSONObject(io { Core.list() })
        val f = json.getJSONArray("folders")
        val n = json.getJSONArray("notes")
        folders = f.map { f.getString(it) }
        notes = n.map { n.getJSONObject(it).let { o -> NoteRef(o.getString("rel"), o.getString("name")) } }
    }

    /** Picks up edits made outside the app (file managers, USB). */
    fun refreshFromDisk() = launch {
        if (!ready) return@launch
        io { Core.rescan() }
        reload()
        val rel = noteRel
        if (rel != null && noteLoaded && noteText == savedText) {
            val text = io { Core.read(rel) }
            if (noteRel == rel && noteText == savedText) {
                noteText = text
                savedText = text
            }
        }
    }

    // ---- Navigation -------------------------------------------------------

    fun navigate(s: Screen) {
        stack.add(s)
        onScreenChanged()
    }

    fun back(): Boolean {
        if (stack.size <= 1) return false
        stack.removeAt(stack.lastIndex)
        onScreenChanged()
        return true
    }

    private fun onScreenChanged() {
        val s = screen
        if (s is Screen.Note) {
            if (s.rel != noteRel) loadNote(s.rel)
        } else {
            saveNow()
        }
    }

    // ---- The open note ----------------------------------------------------

    private fun loadNote(rel: String) {
        saveNow()
        noteRel = rel
        noteLoaded = false
        noteText = ""
        launch {
            val text = io { Core.read(rel) }
            if (noteRel == rel) {
                noteText = text
                savedText = text
                noteLoaded = true
                editing = text.isBlank()
            }
        }
    }

    fun updateText(text: String) {
        noteText = text
        saveJob?.cancel()
        saveJob = viewModelScope.launch {
            delay(800)
            saveNow()
        }
    }

    fun saveNow() {
        launch { flush() }
    }

    private suspend fun flush() {
        saveJob?.cancel()
        val rel = noteRel ?: return
        val text = noteText
        if (!noteLoaded || text == savedText) return
        savedText = text
        io { Core.save(rel, text) }
    }

    // ---- Vault operations -------------------------------------------------

    fun createNote(folder: String, name: String) = launch {
        val rel = io { Core.createNote(folder, name) }
        reload()
        navigate(Screen.Note(rel))
    }

    fun createFolder(parent: String, name: String) = launch {
        val rel = io { Core.createFolder(parent, name) }
        reload()
        navigate(Screen.Browse(rel))
    }

    fun rename(rel: String, name: String) = launch {
        flush()
        val newRel = io { Core.rename(rel, name) }
        fun fix(p: String) = when {
            p == rel -> newRel
            p.startsWith("$rel/") -> newRel + p.removePrefix(rel)
            else -> p
        }
        for (i in stack.indices) {
            stack[i] = when (val s = stack[i]) {
                is Screen.Note -> Screen.Note(fix(s.rel))
                is Screen.Browse -> Screen.Browse(fix(s.folder))
                Screen.Search -> s
            }
        }
        noteRel = noteRel?.let(::fix)
        reload()
        // Links inside the open note may have been rewritten.
        refreshFromDisk()
    }

    fun delete(rel: String) = launch {
        flush()
        io { Core.delete(rel) }
        val gone = { p: String -> p == rel || p.startsWith("$rel/") }
        stack.removeAll {
            (it is Screen.Note && gone(it.rel)) || (it is Screen.Browse && it.folder.isNotEmpty() && gone(it.folder))
        }
        if (stack.isEmpty()) stack.add(Screen.Browse(""))
        if (noteRel?.let(gone) == true) {
            noteRel = null
            noteLoaded = false
        }
        reload()
        onScreenChanged()
    }

    /** Follows a wiki link, creating the note if it doesn't exist yet. */
    fun openLink(target: String) = launch {
        var rel = io { Core.resolve(target) }
        if (rel.isEmpty()) {
            val clean = target.substringBefore('#').trim().removeSuffix(".md")
            if (clean.isEmpty()) return@launch
            rel = io { Core.createNote(parentOf(clean), nameOf(clean)) }
            reload()
        }
        navigate(Screen.Note(rel))
    }

    // ---- Queries ----------------------------------------------------------

    suspend fun preview(text: String): String = io { Core.preview(text) }

    suspend fun backlinks(rel: String): List<NoteRef> {
        val a = JSONArray(io { Core.backlinks(rel) })
        return a.map { a.getJSONObject(it).let { o -> NoteRef(o.getString("rel"), o.getString("name")) } }
    }

    suspend fun outgoing(text: String): List<OutLink> {
        val a = JSONArray(io { Core.outgoing(text) })
        return a.map { a.getJSONObject(it).let { o -> OutLink(o.getString("target"), o.getBoolean("exists")) } }
    }

    suspend fun search(query: String): List<SearchHit> {
        if (query.isBlank()) return emptyList()
        val a = JSONArray(io { Core.search(query) })
        return a.map {
            a.getJSONObject(it).let { o -> SearchHit(o.getString("rel"), o.getString("name"), o.getString("text")) }
        }
    }
}
