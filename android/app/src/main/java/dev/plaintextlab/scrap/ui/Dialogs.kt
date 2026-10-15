package dev.plaintextlab.scrap.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.nameOf
import dev.plaintextlab.scrap.parentOf
import dev.plaintextlab.scrap.titleOf
import kotlinx.coroutines.delay

/** What a name prompt is for. */
sealed interface NameDialog {
    data class NewNote(val folder: String) : NameDialog
    data class NewFolder(val parent: String) : NameDialog
    data class Rename(val rel: String, val current: String) : NameDialog
}

@Composable
fun NameDialogHost(dialog: NameDialog, onDismiss: () -> Unit, onConfirm: (NameDialog, String) -> Unit) {
    val (title, initial, action) = when (dialog) {
        is NameDialog.NewNote -> Triple("New note", "", "Create")
        is NameDialog.NewFolder -> Triple("New folder", "", "Create")
        is NameDialog.Rename -> Triple("Rename", dialog.current, "Rename")
    }
    NamePrompt(title, initial, action, onDismiss) { onConfirm(dialog, it) }
}

@Composable
fun NamePrompt(
    title: String,
    initial: String,
    confirmLabel: String,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    var value by remember { mutableStateOf(TextFieldValue(initial, TextRange(0, initial.length))) }
    val focus = remember { FocusRequester() }
    val submit = { if (value.text.isNotBlank()) onConfirm(value.text.trim()) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                label = { Text("Name") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(
                    capitalization = KeyboardCapitalization.Sentences,
                    imeAction = ImeAction.Done,
                ),
                keyboardActions = KeyboardActions(onDone = { submit() }),
                modifier = Modifier.focusRequester(focus),
            )
        },
        confirmButton = {
            TextButton(onClick = submit, enabled = value.text.isNotBlank()) { Text(confirmLabel) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
    LaunchedEffect(Unit) {
        delay(100)
        runCatching { focus.requestFocus() }
    }
}

/**
 * Picks a destination folder for moving [rel]. Its current folder and, for a folder, the
 * folder itself and everything inside it can't be chosen.
 */
@Composable
fun MoveDialog(rel: String, isFolder: Boolean, folders: List<String>, onDismiss: () -> Unit, onMove: (String) -> Unit) {
    val current = parentOf(rel)
    // Depth-first order, so subfolders sit under their parents.
    val targets = remember(folders) { listOf("") + folders.sortedBy { it.lowercase().replace('/', '\u0000') } }
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(ScrapIcons.Move, contentDescription = null, tint = LocalGruvbox.current.orange) },
        title = { Text("Move “${titleOf(rel)}” to") },
        text = {
            LazyColumn(Modifier.heightIn(max = 360.dp)) {
                items(targets, key = { "dest:$it" }) { folder ->
                    val inside = isFolder && (folder == rel || folder.startsWith("$rel/"))
                    val enabled = folder != current && !inside
                    val depth = if (folder.isEmpty()) 0 else folder.count { it == '/' } + 1
                    ListItem(
                        headlineContent = {
                            Text(if (folder.isEmpty()) "Notes" else nameOf(folder), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        },
                        supportingContent = if (folder == current) ({ Text("Current folder") }) else null,
                        leadingContent = {
                            Icon(
                                if (folder.isEmpty()) ScrapIcons.Note else ScrapIcons.Folder,
                                contentDescription = null,
                                tint = if (folder.isEmpty()) LocalGruvbox.current.blue else LocalGruvbox.current.yellow,
                                modifier = Modifier.padding(start = (16 * maxOf(depth - 1, 0)).dp),
                            )
                        },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                        modifier = Modifier
                            .alpha(if (enabled) 1f else 0.38f)
                            .clickable(enabled = enabled) { onMove(folder) },
                    )
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
fun ConfirmDelete(name: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Default.Delete, contentDescription = null, tint = LocalGruvbox.current.red) },
        title = { Text("Delete “$name”?") },
        text = { Text("It will be moved to the trash, where you can restore it from ⋮ › Trash.") },
        confirmButton = { TextButton(onClick = onConfirm) { Text("Delete") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
