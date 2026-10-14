package dev.plaintextlab.scrap.ui

import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
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
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
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

@Composable
fun ConfirmDelete(name: String, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Default.Delete, contentDescription = null) },
        title = { Text("Delete “$name”?") },
        text = { Text("It will be moved to the vault’s .trash folder.") },
        confirmButton = { TextButton(onClick = onConfirm) { Text("Delete") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
