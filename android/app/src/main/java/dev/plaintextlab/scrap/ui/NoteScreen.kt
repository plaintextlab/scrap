package dev.plaintextlab.scrap.ui

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.Density
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import coil3.compose.AsyncImage
import com.mikepenz.markdown.coil3.Coil3ImageTransformerImpl
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import com.mikepenz.markdown.m3.Markdown
import com.mikepenz.markdown.m3.markdownColor
import dev.plaintextlab.scrap.Attachments
import dev.plaintextlab.scrap.Core
import dev.plaintextlab.scrap.NoteRef
import java.io.File
import dev.plaintextlab.scrap.OutLink
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.titleOf

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteScreen(vm: VaultViewModel, rel: String, snackbar: SnackbarHostState) {
    var menu by remember { mutableStateOf(false) }
    var showLinks by remember { mutableStateOf(false) }
    var renaming by remember { mutableStateOf(false) }
    var deleting by remember { mutableStateOf(false) }
    var moving by remember { mutableStateOf(false) }
    val reading = vm.noteLoaded && !vm.editing
    val context = LocalContext.current

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
                                text = { Text("Share") },
                                leadingIcon = { Icon(Icons.Default.Share, null) },
                                onClick = {
                                    menu = false
                                    val send = Intent(Intent.ACTION_SEND)
                                        .setType("text/plain")
                                        .putExtra(Intent.EXTRA_SUBJECT, titleOf(rel))
                                        // Plain markdown: colours are HTML most apps would show as tags.
                                        .putExtra(Intent.EXTRA_TEXT, Core.stripColors(vm.noteText))
                                    context.startActivity(Intent.createChooser(send, null))
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Rename") },
                                leadingIcon = { Icon(Icons.Default.Edit, null) },
                                onClick = {
                                    menu = false
                                    renaming = true
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Move to…") },
                                leadingIcon = { Icon(ScrapIcons.Move, null, tint = LocalAccents.current.orange) },
                                onClick = {
                                    menu = false
                                    moving = true
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Delete") },
                                leadingIcon = { Icon(Icons.Default.Delete, null, tint = LocalAccents.current.red) },
                                onClick = {
                                    menu = false
                                    deleting = true
                                },
                            )
                            HorizontalDivider()
                            val words = remember(vm.noteText) {
                                vm.noteText.split(Regex("""\s+""")).count { it.isNotEmpty() }
                            }
                            DropdownMenuItem(
                                text = { Text("$words words · ${vm.noteText.length} characters") },
                                onClick = {},
                                enabled = false,
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
            else -> ScaledText(vm.settings.textScale) {
                if (vm.editing) {
                    Editor(vm.noteText, vm::updateText, padding, onAddImage = vm::addImage)
                } else {
                    Reader(vm, rel, vm.noteText, padding)
                }
            }
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
    if (moving) {
        MoveDialog(rel, isFolder = false, folders = vm.folders, onDismiss = { moving = false }) { dest ->
            moving = false
            vm.move(rel, dest)
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
private fun Reader(vm: VaultViewModel, rel: String, text: String, padding: PaddingValues) {
    val blocks by produceState<List<Attachments.Block>?>(null, text, rel) {
        val markdown = vm.preview(text)
        value = withContext(Dispatchers.IO) {
            Attachments.splitImages(Attachments.resolve(markdown, rel, File(vm.vaultPath)))
        }
    }
    val accents = LocalAccents.current
    val annotator = remember(accents) { TextColors.annotator(accents) }
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
        CompositionLocalProvider(LocalUriHandler provides handler) {
            for (block in blocks.orEmpty()) {
                when (block) {
                    is Attachments.Block.Text -> Markdown(
                        content = block.markdown,
                        colors = markdownColor(
                            linkText = LocalAccents.current.blue,
                            codeText = LocalAccents.current.green,
                            codeBackground = MaterialTheme.colorScheme.surfaceContainerHigh,
                        ),
                        imageTransformer = Coil3ImageTransformerImpl,
                        annotator = annotator,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    // Full width at the image's own aspect ratio.
                    is Attachments.Block.Image -> AsyncImage(
                        model = block.url,
                        contentDescription = block.alt.ifEmpty { null },
                        contentScale = ContentScale.FillWidth,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 8.dp)
                            .clip(RoundedCornerShape(12.dp)),
                    )
                }
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
                    leadingContent = { Icon(ScrapIcons.Note, null, tint = LocalAccents.current.blue) },
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
                    leadingContent = { Icon(ScrapIcons.Link, null, tint = LocalAccents.current.aqua) },
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

/** Applies the user's text size to note content only, leaving the app chrome alone. */
@Composable
private fun ScaledText(scale: Float, content: @Composable () -> Unit) {
    val density = LocalDensity.current
    CompositionLocalProvider(LocalDensity provides Density(density.density, density.fontScale * scale)) {
        content()
    }
}
