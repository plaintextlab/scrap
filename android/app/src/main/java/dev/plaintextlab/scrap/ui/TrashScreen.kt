package dev.plaintextlab.scrap.ui

import android.text.format.DateUtils
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberTopAppBarState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.TrashItem
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.parentOf

/** Deleted notes and folders, which can be restored or removed for good. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrashScreen(vm: VaultViewModel, snackbar: SnackbarHostState) {
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior(rememberTopAppBarState())
    var menu by remember { mutableStateOf(false) }
    var emptying by remember { mutableStateOf(false) }
    var purging by remember { mutableStateOf<TrashItem?>(null) }
    LaunchedEffect(Unit) { vm.loadTrash() }

    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = { Text("Trash") },
                navigationIcon = {
                    IconButton(onClick = { vm.back() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    if (vm.trash.isNotEmpty()) {
                        Box {
                            IconButton(onClick = { menu = true }) {
                                Icon(Icons.Default.MoreVert, contentDescription = "More")
                            }
                            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                                DropdownMenuItem(
                                    text = { Text("Empty trash") },
                                    leadingIcon = { Icon(Icons.Default.Delete, null, tint = LocalAccents.current.red) },
                                    onClick = {
                                        menu = false
                                        emptying = true
                                    },
                                )
                            }
                        }
                    }
                },
                scrollBehavior = scroll,
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        if (vm.trash.isEmpty()) {
            Column(
                Modifier.fillMaxSize().padding(padding).padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Icon(
                    Icons.Default.Delete,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(64.dp),
                )
                Spacer(Modifier.height(16.dp))
                Text("Trash is empty", style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.height(4.dp))
                Text(
                    "Deleted notes and folders show up here.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            return@Scaffold
        }
        LazyColumn(contentPadding = PaddingValues(top = padding.calculateTopPadding(), bottom = 32.dp)) {
            items(vm.trash, key = { it.name }) { item ->
                TrashRow(item, onRestore = { vm.restore(item) }, onPurge = { purging = item })
            }
        }
    }

    if (emptying) {
        ConfirmPermanent(
            title = "Empty trash?",
            text = "Everything in the trash will be deleted for good.",
            action = "Empty trash",
            onDismiss = { emptying = false },
        ) {
            emptying = false
            vm.emptyTrash()
        }
    }
    purging?.let { item ->
        ConfirmPermanent(
            title = "Delete “${displayName(item)}” forever?",
            text = "This can't be undone.",
            action = "Delete",
            onDismiss = { purging = null },
        ) {
            purging = null
            vm.deleteForever(item)
        }
    }
}

@Composable
private fun TrashRow(item: TrashItem, onRestore: () -> Unit, onPurge: () -> Unit) {
    var menu by remember { mutableStateOf(false) }
    val from = parentOf(item.original).ifEmpty { "Notes" }
    val ago = if (item.deleted > 0) {
        DateUtils.getRelativeTimeSpanString(item.deleted, System.currentTimeMillis(), DateUtils.MINUTE_IN_MILLIS)
    } else {
        null
    }
    ListItem(
        headlineContent = { Text(displayName(item), maxLines = 1, overflow = TextOverflow.Ellipsis) },
        supportingContent = {
            Text(
                listOfNotNull("From $from", ago?.let { "deleted $it" }).joinToString(" · "),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        },
        leadingContent = { Icon(
            if (item.isDir) ScrapIcons.Folder else ScrapIcons.Note,
            contentDescription = null,
            tint = if (item.isDir) LocalAccents.current.yellow else LocalAccents.current.blue,
        ) },
        trailingContent = {
            Box {
                IconButton(onClick = { menu = true }) {
                    Icon(Icons.Default.MoreVert, contentDescription = "Actions")
                }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    DropdownMenuItem(
                        text = { Text("Restore") },
                        leadingIcon = { Icon(ScrapIcons.Restore, null) },
                        onClick = {
                            menu = false
                            onRestore()
                        },
                    )
                    DropdownMenuItem(
                        text = { Text("Delete forever") },
                        leadingIcon = { Icon(Icons.Default.Delete, null, tint = LocalAccents.current.red) },
                        onClick = {
                            menu = false
                            onPurge()
                        },
                    )
                }
            }
        },
    )
}

/** The name it had before deletion, without ".md". */
private fun displayName(item: TrashItem): String =
    item.original.substringAfterLast('/').let { if (item.isDir) it else it.removeSuffix(".md") }

@Composable
private fun ConfirmPermanent(title: String, text: String, action: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Default.Delete, contentDescription = null, tint = LocalAccents.current.red) },
        title = { Text(title) },
        text = { Text(text) },
        confirmButton = {
            TextButton(onClick = onConfirm) { Text(action, color = MaterialTheme.colorScheme.error) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
