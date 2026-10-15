package dev.plaintextlab.scrap.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.parentOf

/**
 * Search across note names and contents: a floating card over the dimmed notes list,
 * with results dropping down under the search field. Results update on every keystroke;
 * with an empty query it lists recently edited notes. Tapping outside closes it.
 */
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

    Box(Modifier.fillMaxSize()) {
        // Scrim over the notes list; tapping it closes search.
        Box(
            Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.4f))
                .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {
                    vm.closeSearch()
                },
        )
        Column(
            Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .imePadding()
                .padding(horizontal = 12.dp, vertical = 8.dp),
        ) {
            Surface(
                shape = RoundedCornerShape(28.dp),
                color = MaterialTheme.colorScheme.surfaceContainerHigh,
                shadowElevation = 8.dp,
                modifier = Modifier.fillMaxWidth().weight(1f, fill = false),
            ) {
                Column {
                    TextField(
                        value = query,
                        onValueChange = vm::updateSearch,
                        placeholder = { Text("Search notes") },
                        singleLine = true,
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
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                        keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }),
                        colors = TextFieldDefaults.colors(
                            focusedContainerColor = Color.Transparent,
                            unfocusedContainerColor = Color.Transparent,
                            focusedIndicatorColor = Color.Transparent,
                            unfocusedIndicatorColor = Color.Transparent,
                        ),
                        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp).focusRequester(focus),
                    )
                    Results(
                        vm = vm,
                        modifier = Modifier.weight(1f, fill = false).nestedScroll(hideKeyboardOnScroll),
                        onOpen = ::open,
                    )
                }
            }
        }
    }
}

@Composable
private fun Results(vm: VaultViewModel, modifier: Modifier, onOpen: (String) -> Unit) {
    val query = vm.searchQuery
    val hits = vm.searchHits
    when {
        query.isBlank() -> {
            val recent = remember(vm.notes) { vm.notes.sortedByDescending { it.modified }.take(12) }
            if (recent.isEmpty()) return
            HorizontalDivider()
            LazyColumn(modifier) {
                item { SectionLabel("Recent") }
                items(recent, key = { it.rel }) { n ->
                    ResultRow(n.name, parentOf(n.rel), null, "") { onOpen(n.rel) }
                }
            }
        }
        hits.isEmpty() && vm.searchedFor == query -> {
            HorizontalDivider()
            Text(
                "No notes match “${query.trim()}”",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 20.dp),
            )
        }
        hits.isNotEmpty() -> {
            HorizontalDivider()
            LazyColumn(modifier) {
                itemsIndexed(hits, key = { i, h -> "$i:${h.rel}" }) { _, hit ->
                    ResultRow(hit.name, parentOf(hit.rel), hit.text, query) { onOpen(hit.rel) }
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
        leadingContent = { Icon(ScrapIcons.Note, contentDescription = null, tint = LocalGruvbox.current.blue) },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
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
