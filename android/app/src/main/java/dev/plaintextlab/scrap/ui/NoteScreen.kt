package dev.plaintextlab.scrap.ui

import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.mikepenz.markdown.m3.Markdown
import dev.plaintextlab.scrap.NoteRef
import dev.plaintextlab.scrap.OutLink
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.titleOf
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteScreen(vm: VaultViewModel, rel: String, snackbar: SnackbarHostState) {
    var menu by remember { mutableStateOf(false) }
    var showLinks by remember { mutableStateOf(false) }
    var renaming by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf(false) }
    val reading = vm.noteLoaded && !vm.editing

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(titleOf(rel), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(onClick = { vm.back() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    if (vm.editing) {
                        IconButton(onClick = { vm.editing = false }) {
                            Icon(Icons.Default.Check, contentDescription = "Done")
                        }
                    }
                    IconButton(onClick = { showLinks = true }) {
                        Icon(ScrapIcons.Link, contentDescription = "Links")
                    }
                    Box {
                        IconButton(onClick = { menu = true }) {
                            Icon(Icons.Default.MoreVert, contentDescription = "More")
                        }
                        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                            DropdownMenuItem(
                                text = { Text("Rename") },
                                leadingIcon = { Icon(Icons.Default.Edit, null) },
                                onClick = {
                                    menu = false
                                    renaming = true
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Delete") },
                                leadingIcon = { Icon(Icons.Default.Delete, null) },
                                onClick = {
                                    menu = false
                                    deleting = true
                                },
                            )
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
            )
        },
        floatingActionButton = {
            if (reading) {
                FloatingActionButton(onClick = { vm.editing = true }) {
                    Icon(Icons.Default.Edit, contentDescription = "Edit")
                }
            }
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            !vm.noteLoaded -> Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
            vm.editing -> Editor(vm.noteText, vm::updateText, padding)
            else -> Reader(vm, vm.noteText, padding)
        }
    }

    if (showLinks) {
        LinksSheet(vm, rel, vm.noteText, onDismiss = { showLinks = false })
    }
    if (renaming) {
        NamePrompt("Rename", titleOf(rel), "Rename", onDismiss = { renaming = false }) {
            renaming = false
            vm.rename(rel, it)
        }
    }
    if (deleting) {
        ConfirmDelete(titleOf(rel), onDismiss = { deleting = false }) {
            deleting = false
            vm.delete(rel)
        }
    }
}

@Composable
private fun Editor(text: String, onChange: (String) -> Unit, padding: PaddingValues) {
    val colors = MaterialTheme.colorScheme
    val highlight = remember(colors) { MarkdownHighlight(colors) }
    val focus = remember { FocusRequester() }
    BasicTextField(
        value = text,
        onValueChange = onChange,
        textStyle = MaterialTheme.typography.bodyLarge.copy(color = colors.onSurface),
        cursorBrush = SolidColor(colors.primary),
        visualTransformation = highlight,
        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
        modifier = Modifier
            .fillMaxSize()
            .padding(padding)
            .consumeWindowInsets(padding)
            .imePadding()
            .padding(horizontal = 20.dp, vertical = 12.dp)
            .focusRequester(focus),
    )
    LaunchedEffect(Unit) {
        delay(100)
        runCatching { focus.requestFocus() }
    }
}

@Composable
private fun Reader(vm: VaultViewModel, text: String, padding: PaddingValues) {
    val markdown by produceState<String?>(null, text) { value = vm.preview(text) }
    val system = LocalUriHandler.current
    val handler = remember(system) {
        object : UriHandler {
            override fun openUri(uri: String) {
                if (uri.startsWith("wiki:")) {
                    vm.openLink(Uri.decode(uri.removePrefix("wiki:")))
                } else {
                    runCatching { system.openUri(uri) }
                }
            }
        }
    }
    Column(
        Modifier
            .fillMaxSize()
            .padding(padding)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 12.dp),
    ) {
        if (text.isBlank()) {
            Text(
                "This note is empty. Tap the pencil to start writing.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.clickable { vm.editing = true },
            )
        }
        markdown?.let {
            CompositionLocalProvider(LocalUriHandler provides handler) {
                Markdown(content = it, modifier = Modifier.fillMaxWidth())
            }
        }
        Spacer(Modifier.height(96.dp))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LinksSheet(vm: VaultViewModel, rel: String, text: String, onDismiss: () -> Unit) {
    val backlinks by produceState(emptyList<NoteRef>(), rel) { value = vm.backlinks(rel) }
    val outgoing by produceState(emptyList<OutLink>(), text) { value = vm.outgoing(text) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.verticalScroll(rememberScrollState())) {
            SectionHeader("Backlinks")
            if (backlinks.isEmpty()) EmptyLine("No notes link here yet.")
            for (b in backlinks) {
                ListItem(
                    headlineContent = { Text(b.name) },
                    supportingContent = { Text(b.rel, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    leadingContent = { Icon(ScrapIcons.Note, null) },
                    modifier = Modifier.clickable {
                        onDismiss()
                        vm.navigate(Screen.Note(b.rel))
                    },
                )
            }
            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            SectionHeader("Links in this note")
            if (outgoing.isEmpty()) EmptyLine("Link to other notes with [[Note name]].")
            for (o in outgoing) {
                ListItem(
                    headlineContent = { Text(o.target) },
                    supportingContent = if (o.exists) null else ({ Text("Not created yet, tap to create") }),
                    leadingContent = { Icon(ScrapIcons.Link, null) },
                    modifier = Modifier.clickable {
                        onDismiss()
                        vm.openLink(o.target)
                    },
                )
            }
            Spacer(Modifier.navigationBarsPadding().height(16.dp))
        }
    }
}

@Composable
private fun SectionHeader(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
}

@Composable
private fun EmptyLine(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
    )
}
