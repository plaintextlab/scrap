package dev.plaintextlab.scrap.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SearchBar
import androidx.compose.material3.SearchBarDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.parentOf

/**
 * Search across note names and contents, shown over the notes list. Results update on
 * every keystroke; with an empty query it lists recently edited notes.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchOverlay(vm: VaultViewModel) {
    val query = vm.searchQuery
    val focus = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current
    // Don't pop the keyboard back up when returning from a result.
    LaunchedEffect(Unit) { if (query.isEmpty()) focus.requestFocus() }
    BackHandler { vm.closeSearch() }

    val hideKeyboardOnScroll = remember(keyboard) {
        object : NestedScrollConnection {
            override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
                if (source == NestedScrollSource.UserInput && available.y != 0f) keyboard?.hide()
                return Offset.Zero
            }
        }
    }
    fun open(rel: String) {
        keyboard?.hide()
        vm.navigate(Screen.Note(rel))
    }

    SearchBar(
        inputField = {
            SearchBarDefaults.InputField(
                query = query,
                onQueryChange = vm::updateSearch,
                onSearch = { keyboard?.hide() },
                expanded = true,
                onExpandedChange = { if (!it) vm.closeSearch() },
                placeholder = { Text("Search notes") },
                leadingIcon = {
                    IconButton(onClick = { vm.closeSearch() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Close search")
                    }
                },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = {
                            vm.updateSearch("")
                            focus.requestFocus()
                        }) {
                            Icon(Icons.Default.Close, contentDescription = "Clear")
                        }
                    }
                },
                modifier = Modifier.focusRequester(focus),
            )
        },
        expanded = true,
        onExpandedChange = { if (!it) vm.closeSearch() },
        modifier = Modifier.fillMaxSize(),
    ) {
        val hits = vm.searchHits
        val list = Modifier.fillMaxSize().imePadding().nestedScroll(hideKeyboardOnScroll)
        when {
            query.isBlank() -> {
                val recent = remember(vm.notes) { vm.notes.sortedByDescending { it.modified }.take(12) }
                LazyColumn(list) {
                    if (recent.isNotEmpty()) item { SectionLabel("Recent") }
                    items(recent, key = { it.rel }) { n ->
                        ResultRow(n.name, parentOf(n.rel), null, "") { open(n.rel) }
                    }
                }
            }
            hits.isEmpty() && vm.searchedFor == query -> Box(list.padding(32.dp), contentAlignment = Alignment.TopCenter) {
                Text(
                    "No notes match “${query.trim()}”",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            else -> LazyColumn(list) {
                itemsIndexed(hits, key = { i, h -> "$i:${h.rel}" }) { _, hit ->
                    ResultRow(hit.name, parentOf(hit.rel), hit.text, query) { open(hit.rel) }
                }
            }
        }
    }
}

@Composable
private fun ResultRow(name: String, folder: String, text: String?, query: String, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(highlight(name, query), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        overlineContent = if (folder.isEmpty()) null else ({
            Text(folder, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }),
        supportingContent = if (text.isNullOrEmpty()) null else ({
            Text(highlight(text, query), maxLines = 2, overflow = TextOverflow.Ellipsis)
        }),
        leadingContent = { Icon(ScrapIcons.Note, contentDescription = null) },
        colors = ListItemDefaults.colors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
    )
}

@Composable
private fun SectionLabel(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelLarge,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 16.dp, top = 12.dp, bottom = 4.dp),
    )
}

/** `text` with every case-insensitive occurrence of `query` emphasised. */
@Composable
private fun highlight(text: String, query: String): AnnotatedString {
    val color = MaterialTheme.colorScheme.primary
    return remember(text, query, color) {
        buildAnnotatedString {
            append(text)
            val q = query.trim()
            if (q.isEmpty()) return@buildAnnotatedString
            val style = SpanStyle(color = color, fontWeight = FontWeight.SemiBold)
            var i = text.indexOf(q, ignoreCase = true)
            while (i >= 0) {
                addStyle(style, i, i + q.length)
                i = text.indexOf(q, i + q.length, ignoreCase = true)
            }
        }
    }
}
