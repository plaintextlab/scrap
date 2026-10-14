package dev.plaintextlab.scrap.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberTopAppBarState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.nameOf
import dev.plaintextlab.scrap.parentOf
import dev.plaintextlab.scrap.titleOf

@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun BrowseScreen(vm: VaultViewModel, folder: String, snackbar: SnackbarHostState) {
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior(rememberTopAppBarState())
    var dialog by remember { mutableStateOf<NameDialog?>(null) }
    var actionsFor by remember { mutableStateOf<String?>(null) }
    var deleting by remember { mutableStateOf<String?>(null) }
    var menu by remember { mutableStateOf(false) }

    val subfolders = vm.folders.filter { parentOf(it) == folder }
    val notes = vm.notes.filter { it.folder == folder }

    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = {
                    Text(if (folder.isEmpty()) "Notes" else nameOf(folder), maxLines = 1, overflow = TextOverflow.Ellipsis)
                },
                navigationIcon = {
                    if (folder.isNotEmpty()) {
                        IconButton(onClick = { vm.back() }) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                },
                actions = {
                    IconButton(onClick = { vm.navigate(Screen.Search) }) {
                        Icon(Icons.Default.Search, contentDescription = "Search")
                    }
                    IconButton(onClick = { dialog = NameDialog.NewFolder(folder) }) {
                        Icon(ScrapIcons.NewFolder, contentDescription = "New folder")
                    }
                    if (folder.isNotEmpty()) {
                        Box {
                            IconButton(onClick = { menu = true }) {
                                Icon(Icons.Default.MoreVert, contentDescription = "More")
                            }
                            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                                DropdownMenuItem(
                                    text = { Text("Rename folder") },
                                    leadingIcon = { Icon(Icons.Default.Edit, null) },
                                    onClick = {
                                        menu = false
                                        dialog = NameDialog.Rename(folder, nameOf(folder))
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text("Delete folder") },
                                    leadingIcon = { Icon(Icons.Default.Delete, null) },
                                    onClick = {
                                        menu = false
                                        deleting = folder
                                    },
                                )
                            }
                        }
                    }
                },
                scrollBehavior = scroll,
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = { dialog = NameDialog.NewNote(folder) },
                icon = { Icon(Icons.Default.Add, contentDescription = null) },
                text = { Text("New note") },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        when {
            !vm.ready -> Box(Modifier.fillMaxSize().padding(padding), contentAlignment = Alignment.Center) {
                CircularProgressIndicator()
            }
            subfolders.isEmpty() && notes.isEmpty() -> EmptyFolder(Modifier.padding(padding))
            else -> LazyColumn(
                contentPadding = PaddingValues(
                    top = padding.calculateTopPadding(),
                    bottom = padding.calculateBottomPadding() + 88.dp,
                ),
            ) {
                items(subfolders, key = { "folder:$it" }) { f ->
                    ListItem(
                        headlineContent = { Text(nameOf(f)) },
                        leadingContent = { Icon(ScrapIcons.Folder, contentDescription = null) },
                        modifier = Modifier.combinedClickable(
                            onClick = { vm.navigate(Screen.Browse(f)) },
                            onLongClick = { actionsFor = f },
                        ),
                    )
                }
                items(notes, key = { it.rel }) { n ->
                    ListItem(
                        headlineContent = { Text(n.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        leadingContent = { Icon(ScrapIcons.Note, contentDescription = null) },
                        modifier = Modifier.combinedClickable(
                            onClick = { vm.navigate(Screen.Note(n.rel)) },
                            onLongClick = { actionsFor = n.rel },
                        ),
                    )
                }
            }
        }
    }

    actionsFor?.let { rel ->
        ModalBottomSheet(onDismissRequest = { actionsFor = null }) {
            Text(
                titleOf(rel),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            ListItem(
                headlineContent = { Text("Rename") },
                leadingContent = { Icon(Icons.Default.Edit, null) },
                modifier = Modifier.combinedClickable(onClick = {
                    actionsFor = null
                    dialog = NameDialog.Rename(rel, titleOf(rel))
                }),
            )
            ListItem(
                headlineContent = { Text("Delete") },
                leadingContent = { Icon(Icons.Default.Delete, null) },
                modifier = Modifier.combinedClickable(onClick = {
                    actionsFor = null
                    deleting = rel
                }),
            )
            Spacer(Modifier.navigationBarsPadding().height(16.dp))
        }
    }

    dialog?.let { d ->
        NameDialogHost(d, onDismiss = { dialog = null }) { which, name ->
            dialog = null
            when (which) {
                is NameDialog.NewNote -> vm.createNote(which.folder, name)
                is NameDialog.NewFolder -> vm.createFolder(which.parent, name)
                is NameDialog.Rename -> vm.rename(which.rel, name)
            }
        }
    }

    deleting?.let { rel ->
        ConfirmDelete(titleOf(rel), onDismiss = { deleting = null }) {
            deleting = null
            vm.delete(rel)
        }
    }
}

@Composable
private fun EmptyFolder(modifier: Modifier) {
    Column(
        modifier.fillMaxSize().padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            ScrapIcons.Empty,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.size(64.dp),
        )
        Spacer(Modifier.height(16.dp))
        Text("Nothing here yet", style = MaterialTheme.typography.titleMedium)
        Spacer(Modifier.height(4.dp))
        Text(
            "Tap New note to start writing.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
