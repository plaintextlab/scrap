package dev.plaintextlab.scrap.widget

import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.lifecycle.lifecycleScope
import dev.plaintextlab.scrap.Core
import dev.plaintextlab.scrap.Lists
import dev.plaintextlab.scrap.SettingsStore
import dev.plaintextlab.scrap.Vault
import dev.plaintextlab.scrap.parentOf
import dev.plaintextlab.scrap.titleOf
import dev.plaintextlab.scrap.ui.LocalAccents
import dev.plaintextlab.scrap.ui.NamePrompt
import dev.plaintextlab.scrap.ui.ScrapIcons
import dev.plaintextlab.scrap.ui.ScrapTheme
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Picks the note a new (or long-pressed) widget shows, or makes a new checklist for it. */
class NoteWidgetConfigActivity : ComponentActivity() {
    private var widgetId = AppWidgetManager.INVALID_APPWIDGET_ID

    @OptIn(ExperimentalMaterial3Api::class)
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        widgetId = intent?.getIntExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID)
            ?: AppWidgetManager.INVALID_APPWIDGET_ID
        // Backing out leaves the widget unplaced.
        setResult(RESULT_CANCELED, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
        if (widgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        val store = SettingsStore(this)
        val repo = store.activeRepoId()
        setContent {
            ScrapTheme(store.load()) {
                val notes by produceState<List<String>?>(null) { value = withContext(Dispatchers.IO) { listNotes(Vault.dir(this@NoteWidgetConfigActivity)) } }
                var naming by remember { mutableStateOf(false) }
                val scope = rememberCoroutineScope()
                Scaffold(topBar = { TopAppBar(title = { Text("Show a note") }) }) { padding ->
                    LazyColumn(Modifier.fillMaxSize().padding(padding)) {
                        item {
                            ListItem(
                                headlineContent = { Text("New checklist…") },
                                supportingContent = { Text("A numbered list to tick off, such as groceries") },
                                leadingContent = { Icon(Icons.Default.Add, null, tint = LocalAccents.current.green) },
                                modifier = Modifier.clickable { naming = true },
                            )
                            HorizontalDivider()
                        }
                        items(notes.orEmpty(), key = { it }) { rel ->
                            ListItem(
                                headlineContent = { Text(titleOf(rel)) },
                                supportingContent = if (parentOf(rel).isEmpty()) null else ({ Text(parentOf(rel)) }),
                                leadingContent = { Icon(ScrapIcons.Note, null, tint = LocalAccents.current.blue) },
                                modifier = Modifier.clickable { choose(repo, rel) },
                            )
                        }
                        if (notes?.isEmpty() == true) {
                            item { ListItem(headlineContent = { Text("No notes yet") }) }
                        }
                    }
                }
                if (naming) {
                    NamePrompt("New checklist", "", "Create", onDismiss = { naming = false }) { name ->
                        naming = false
                        scope.launch {
                            val rel = withContext(Dispatchers.IO) {
                                runCatching {
                                    Vault.ensureOpen(this@NoteWidgetConfigActivity)
                                    Core.createNote("", name).also { Core.save(it, Lists.NEW_CHECKLIST) }
                                }.getOrNull()
                            }
                            if (rel != null) choose(repo, rel)
                        }
                    }
                }
            }
        }
    }

    private fun choose(repo: String, rel: String) {
        WidgetPrefs.set(this, widgetId, WidgetPrefs.Target(repo, rel))
        lifecycleScope.launch {
            runCatching {
                val glanceId = GlanceAppWidgetManager(this@NoteWidgetConfigActivity).getGlanceIdBy(widgetId)
                NoteWidget().update(this@NoteWidgetConfigActivity, glanceId)
            }
            setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
            finish()
        }
    }

    /** Notes in the vault, most recently changed first; hidden folders (.git, .trash, …) skipped. */
    private fun listNotes(root: File): List<String> =
        root.walkTopDown()
            .onEnter { it == root || !it.name.startsWith(".") }
            .filter { it.isFile && it.name.endsWith(".md") }
            .sortedByDescending { it.lastModified() }
            .map { it.relativeTo(root).invariantSeparatorsPath }
            .toList()
}
