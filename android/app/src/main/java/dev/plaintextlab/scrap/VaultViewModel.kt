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

class VaultViewModel(private val app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)

    /** Navigation back stack; the last entry is on screen. */
    val stack = mutableStateListOf<Screen>(Screen.Browse(""))
    val screen: Screen get() = stack.last()

    var settings by mutableStateOf(store.load())
        private set

    /** All repositories (each its own folder of notes) and which one is open. */
    var repos by mutableStateOf(store.repos())
        private set
    var activeRepoId by mutableStateOf(store.activeRepoId())
        private set
    val activeRepo: Repo get() = repos.firstOrNull { it.id == activeRepoId } ?: repos.first()

    /** Name under the launcher icon (see [LauncherName]). */
    var appName by mutableStateOf(LauncherName.current(app))
        private set
    var ready by mutableStateOf(false)
        private set
    var folders by mutableStateOf(emptyList<String>())
        private set
    var notes by mutableStateOf(emptyList<NoteRef>())
        private set

    /** One-off message for the snackbar. */
    var message by mutableStateOf<String?>(null)

    var syncing by mutableStateOf(false)
        private set
    var syncStatus by mutableStateOf(store.lastSync())
        private set

    /** Something changed locally since the last successful sync. */
    private var unsynced = false

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
        launch {
            io { Vault.ensureOpen(app) }
            reload()
            ready = true
            if (settings.sync.configured && settings.sync.auto) sync(quiet = true)
        }
    }

    private fun launch(block: suspend CoroutineScope.() -> Unit) = viewModelScope.launch {
        try {
            block()
        } catch (e: Exception) {
            message = e.message ?: "Something went wrong"
        }
    }

    private suspend fun <T> io(block: () -> T): T = withContext(Dispatchers.IO) { block() }

    private suspend fun reload() {
        val json = JSONObject(io { Core.list() })
        val f = json.getJSONArray("folders")
        val n = json.getJSONArray("notes")
        folders = f.map { f.getString(it) }
        notes = n.map {
            n.getJSONObject(it).let { o -> NoteRef(o.getString("rel"), o.getString("name"), o.optLong("modified")) }
        }
        // Notes changed (sync, rename, delete): refresh open search results.
        if (searchOpen && searchQuery.isNotBlank()) updateSearch(searchQuery)
    }

    /** Picks up changes made outside the app (sync, file managers, USB). */
    private suspend fun reloadFromDisk() {
        io { Core.rescan() }
        reload()
        val rel = noteRel
        if (rel != null && noteLoaded && noteText == savedText) {
            val text = runCatching { io { Core.read(rel) } }.getOrNull()
            if (noteRel == rel && noteText == savedText) {
                if (text == null) {
                    // Deleted elsewhere.
                    stack.removeAll { it is Screen.Note && it.rel == rel }
                    if (stack.isEmpty()) stack.add(Screen.Browse(""))
                    noteRel = null
                    noteLoaded = false
                } else {
                    noteText = text
                    savedText = text
                }
            }
        }
    }

    // ---- Lifecycle --------------------------------------------------------

    fun onForeground() {
        if (!ready) return
        launch {
            reloadFromDisk()
            val last = store.lastSync()?.time ?: 0
            val s = settings.sync
            if (s.configured && s.auto && System.currentTimeMillis() - last > 60_000) sync(quiet = true)
        }
    }

    fun onBackground() {
        launch {
            flush()
            val s = settings.sync
            // The process may be killed once we're in the background; let WorkManager push.
            if (unsynced && s.configured && s.auto) {
                Sync.soon(app)
                unsynced = false
            }
        }
    }

    // ---- Settings ---------------------------------------------------------

    fun updateSettings(change: (AppSettings) -> AppSettings) {
        settings = change(settings)
        store.save(settings)
        repos = store.repos()
        Sync.schedulePeriodic(app, settings.sync.configured && settings.sync.background)
    }

    /** Switches between Gruvbox dark and light, starting from whatever is showing now. */
    fun toggleDark(currentlyDark: Boolean) {
        updateSettings { it.copy(theme = if (currentlyDark) ThemeMode.Light else ThemeMode.Dark) }
    }

    fun changeAppName(name: String) {
        if (name == appName) return
        LauncherName.set(app, name)
        appName = name
        message = "The launcher will show “$name” in a moment"
    }

    /** Saves the open repository's connection and runs a sync; returns to Settings on success. */
    fun connect(repo: Repo, token: String?) {
        if (!token.isNullOrBlank()) store.setToken(token.trim())
        store.saveRepo(repo.copy(id = activeRepoId))
        settings = store.load()
        repos = store.repos()
        Sync.schedulePeriodic(app, settings.sync.configured && settings.sync.background)
        launch {
            if (sync(quiet = false) && screen is Screen.SyncSetup) back()
        }
    }

    fun disconnect() {
        store.setToken(null)
        store.clearSyncStatus()
        syncStatus = null
        updateSettings { it.copy(sync = SyncSettings(auto = it.sync.auto, background = it.sync.background)) }
    }

    // ---- Repositories -----------------------------------------------------

    /**
     * Adds a repository with its own (empty) folder, switches to it and downloads its notes.
     * Returns to Settings once the first sync succeeded.
     */
    fun addRepo(repo: Repo, token: String) {
        val id = java.util.UUID.randomUUID().toString().take(8)
        store.saveRepo(repo.copy(id = id))
        store.setToken(token.trim(), id)
        repos = store.repos()
        launch {
            if (!switchTo(id)) return@launch
            if (sync(quiet = false)) {
                stack.removeAll { it is Screen.SyncSetup }
                if (stack.isEmpty()) stack.add(Screen.Browse(""))
            }
        }
    }

    fun switchRepo(id: String) {
        launch {
            if (switchTo(id) && settings.sync.configured && settings.sync.auto) sync(quiet = true)
        }
    }

    /**
     * Opens another repository's folder. Changes in the one being left are synced in the
     * background so they don't wait until it's opened again.
     */
    private suspend fun switchTo(id: String): Boolean {
        if (id == activeRepoId) return false
        if (syncing) {
            message = "Wait for the sync to finish, then switch"
            return false
        }
        flush()
        val leaving = activeRepo
        val pushLeaving = unsynced && settings.sync.configured
        store.setActiveRepo(id)
        if (pushLeaving) {
            viewModelScope.launch {
                try {
                    Sync.run(app, leaving.id)
                } catch (e: Exception) {
                    message = "Couldn't sync ${leaving.label}: ${e.message}"
                }
            }
        }
        unsynced = false
        activeRepoId = id
        settings = store.load()
        syncStatus = store.lastSync()
        Sync.schedulePeriodic(app, settings.sync.configured && settings.sync.background)
        // Nothing from the previous repository stays on screen.
        saveJob?.cancel()
        noteRel = null
        noteLoaded = false
        noteText = ""
        savedText = ""
        editing = false
        closeSearch()
        stack.clear()
        stack.add(Screen.Browse(""))
        io { Vault.ensureOpen(app) }
        reload()
        message = "Switched to ${activeRepo.label}"
        return true
    }

    /** Removes a repository that isn't open, deleting its folder on this phone. */
    fun removeRepo(id: String) {
        if (id == activeRepoId) return
        val repo = repos.firstOrNull { it.id == id } ?: return
        launch {
            io { Vault.dir(app, id).deleteRecursively() }
            store.removeRepo(id)
            repos = store.repos()
            message = "Removed ${repo.label}"
        }
    }

    fun lastSync(id: String): SyncStatus? = store.lastSync(id)

    fun repoConfigured(id: String): Boolean =
        repos.firstOrNull { it.id == id }?.url?.isNotBlank() == true && store.hasToken(id)

    // ---- Sync -------------------------------------------------------------

    fun syncNow() {
        launch { sync(quiet = false) }
    }

    /** Pull-to-refresh: sync if set up, otherwise just re-read the folder. */
    fun refresh() {
        launch {
            if (settings.sync.configured) sync(quiet = false) else reloadFromDisk()
        }
    }

    /** Returns whether the sync succeeded. Quiet syncs only report failures. */
    private suspend fun sync(quiet: Boolean): Boolean {
        if (!settings.sync.configured) {
            if (!quiet) message = "Set up GitHub sync in Settings first"
            return false
        }
        if (syncing) return false
        syncing = true
        return try {
            flush()
            val summary = Sync.run(app)
            unsynced = false
            reloadFromDisk()
            if (!quiet || summary != "Already up to date") message = summary
            true
        } catch (e: Exception) {
            message = "Sync failed: ${e.message}"
            false
        } finally {
            syncing = false
            syncStatus = store.lastSync()
        }
    }

    // ---- Navigation -------------------------------------------------------

    fun navigate(s: Screen) {
        if (s == screen) return
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
                editing = text.isBlank() || settings.openInEdit
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
        unsynced = true
        notes = notes.map { if (it.rel == rel) it.copy(modified = System.currentTimeMillis()) else it }
    }

    // ---- Vault operations -------------------------------------------------

    fun createNote(folder: String, name: String) = launch {
        val rel = io { Core.createNote(folder, name) }
        unsynced = true
        reload()
        navigate(Screen.Note(rel))
    }

    fun createFolder(parent: String, name: String) = launch {
        val rel = io { Core.createFolder(parent, name) }
        reload()
        navigate(Screen.Browse(rel))
    }

    /** Text shared from another app becomes a new note. */
    fun importShared(text: String, subject: String?) = launch {
        while (!ready) delay(50)
        val title = subject?.takeIf { it.isNotBlank() }
            ?: text.lineSequence().firstOrNull { it.isNotBlank() }?.trim()?.trimStart('#', ' ')?.take(60)
            ?: "Shared note"
        val rel = io {
            Core.createNote("", title).also { Core.save(it, text) }
        }
        unsynced = true
        reload()
        stack.removeAll { it !is Screen.Browse || it.folder.isNotEmpty() }
        if (stack.isEmpty()) stack.add(Screen.Browse(""))
        navigate(Screen.Note(rel))
    }

    fun rename(rel: String, name: String) = launch {
        flush()
        relocated(rel, io { Core.rename(rel, name) })
    }

    fun move(rel: String, folder: String) = launch {
        flush()
        val newRel = io { Core.move(rel, folder) }
        if (newRel == rel) return@launch
        relocated(rel, newRel)
        message = "Moved to ${if (folder.isEmpty()) "Notes" else nameOf(folder)}"
    }

    /** Points open screens at a note or folder's new path after a rename or move. */
    private suspend fun relocated(rel: String, newRel: String) {
        unsynced = true
        fun fix(p: String) = when {
            p == rel -> newRel
            p.startsWith("$rel/") -> newRel + p.removePrefix(rel)
            else -> p
        }
        for (i in stack.indices) {
            stack[i] = when (val s = stack[i]) {
                is Screen.Note -> Screen.Note(fix(s.rel))
                is Screen.Browse -> Screen.Browse(fix(s.folder))
                else -> s
            }
        }
        noteRel = noteRel?.let(::fix)
        // Links inside the open note may have been rewritten.
        reloadFromDisk()
    }

    fun delete(rel: String) = launch {
        flush()
        io { Core.delete(rel) }
        unsynced = true
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

    // ---- Trash ------------------------------------------------------------

    var trash by mutableStateOf(emptyList<TrashItem>())
        private set

    fun loadTrash() = launch {
        val a = JSONArray(io { Core.trash() })
        trash = a.map {
            a.getJSONObject(it).let { o ->
                TrashItem(o.getString("name"), o.getString("original"), o.getBoolean("dir"), o.optLong("deleted"))
            }
        }
    }

    fun restore(item: TrashItem) = launch {
        val rel = io { Core.restore(item.name) }
        unsynced = true
        reload()
        loadTrash()
        message = "Restored to ${if (parentOf(rel).isEmpty()) "Notes" else parentOf(rel)}"
    }

    fun deleteForever(item: TrashItem) = launch {
        io { Core.deleteForever(item.name) }
        loadTrash()
    }

    fun emptyTrash() = launch {
        io { Core.emptyTrash() }
        loadTrash()
    }

    // ---- Images -----------------------------------------------------------

    /**
     * Copies a picked or captured image into the vault's attachments folder and returns
     * markdown that embeds it in the open note, or null if it couldn't be read.
     */
    suspend fun addImage(source: android.net.Uri): String? {
        val rel = noteRel ?: return null
        val saved = io { Attachments.save(app, Vault.dir(app), source) } ?: return null
        unsynced = true
        return "![](${Attachments.relativeTo(rel, saved)})"
    }

    /** Follows a wiki link, creating the note if it doesn't exist yet. */
    fun openLink(target: String) = launch {
        var rel = io { Core.resolve(target) }
        if (rel.isEmpty()) {
            val clean = target.substringBefore('#').trim().removeSuffix(".md")
            if (clean.isEmpty()) return@launch
            rel = io { Core.createNote(parentOf(clean), nameOf(clean)) }
            unsynced = true
            reload()
        }
        navigate(Screen.Note(rel))
    }

    // ---- Queries ----------------------------------------------------------

    val vaultPath: String get() = Vault.dir(app).path

    suspend fun preview(text: String): String = io { Core.preview(text) }

    suspend fun backlinks(rel: String): List<NoteRef> {
        val a = JSONArray(io { Core.backlinks(rel) })
        return a.map { a.getJSONObject(it).let { o -> NoteRef(o.getString("rel"), o.getString("name")) } }
    }

    suspend fun outgoing(text: String): List<OutLink> {
        val a = JSONArray(io { Core.outgoing(text) })
        return a.map { a.getJSONObject(it).let { o -> OutLink(o.getString("target"), o.getBoolean("exists")) } }
    }

    private suspend fun search(query: String): List<SearchHit> {
        if (query.isBlank()) return emptyList()
        val a = JSONArray(io { Core.search(query) })
        return a.map {
            a.getJSONObject(it).let { o -> SearchHit(o.getString("rel"), o.getString("name"), o.getString("text")) }
        }
    }

    // ---- Search overlay ---------------------------------------------------
    // Kept here rather than in the UI so the query and results survive opening a result
    // and coming back.

    var searchOpen by mutableStateOf(false)
        private set
    var searchQuery by mutableStateOf("")
        private set
    var searchHits by mutableStateOf(emptyList<SearchHit>())
        private set
    /** The query `searchHits` belongs to; differs from `searchQuery` while a search runs. */
    var searchedFor by mutableStateOf("")
        private set
    private var searchJob: Job? = null

    fun openSearch() {
        searchOpen = true
    }

    fun closeSearch() {
        searchOpen = false
        updateSearch("")
    }

    /** Searches on every keystroke; the previous results stay up until new ones arrive. */
    fun updateSearch(query: String) {
        searchQuery = query
        searchJob?.cancel()
        if (query.isBlank()) {
            searchHits = emptyList()
            searchedFor = query
            return
        }
        searchJob = viewModelScope.launch {
            val hits = try {
                search(query)
            } catch (e: java.io.IOException) {
                emptyList()
            }
            if (searchQuery == query) {
                searchHits = hits
                searchedFor = query
            }
        }
    }
}
