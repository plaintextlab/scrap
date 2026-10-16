package dev.plaintextlab.scrap.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import dev.plaintextlab.scrap.Repo
import dev.plaintextlab.scrap.SyncSettings
import dev.plaintextlab.scrap.VaultViewModel
import dev.plaintextlab.scrap.normalizeRepoUrl

private const val NEW_TOKEN_URL = "https://github.com/settings/personal-access-tokens/new"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SyncSetupScreen(vm: VaultViewModel, snackbar: SnackbarHostState, adding: Boolean) {
    val current = if (adding) SyncSettings() else vm.settings.sync
    val hasToken = !adding && current.hasToken
    val uri = LocalUriHandler.current
    var label by rememberSaveable { mutableStateOf(if (adding) "" else vm.activeRepo.name) }
    var url by rememberSaveable { mutableStateOf(current.url) }
    var username by rememberSaveable { mutableStateOf(current.username) }
    var token by rememberSaveable { mutableStateOf("") }
    var showToken by rememberSaveable { mutableStateOf(false) }
    var branch by rememberSaveable { mutableStateOf(current.branch) }
    var name by rememberSaveable { mutableStateOf(current.authorName) }
    var email by rememberSaveable { mutableStateOf(current.authorEmail) }
    var confirmDisconnect by rememberSaveable { mutableStateOf(false) }

    val repo = normalizeRepoUrl(url)
    val urlError = url.isNotBlank() && repo == null
    val canSave = repo != null && (token.isNotBlank() || hasToken) && !vm.syncing

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (adding) "Add repository" else "GitHub sync") },
                navigationIcon = {
                    IconButton(onClick = { vm.back() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .consumeWindowInsets(padding)
                .imePadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 8.dp)
                .navigationBarsPadding(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                if (adding) {
                    "Each repository is its own set of notes, kept in its own folder on this phone. " +
                        "Its notes are downloaded once it's added; switch between repositories from the " +
                        "title on the notes screen or in Settings."
                } else {
                    "Your notes are synced with a GitHub repository over HTTPS. Use a private repository and a " +
                        "fine-grained personal access token with read and write access to its contents."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            OutlinedTextField(
                value = url,
                onValueChange = { url = it },
                label = { Text("Repository") },
                placeholder = { Text("you/notes or https://github.com/you/notes.git") },
                isError = urlError,
                supportingText = {
                    Text(if (urlError) "Use the repository's HTTPS address (SSH isn't supported)" else repo ?: "")
                },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, autoCorrectEnabled = false),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = label,
                onValueChange = { label = it },
                label = { Text("Name (optional)") },
                placeholder = { Text(repo?.let { Repo.repoName(it) }?.ifBlank { null } ?: "Shown in the repository switcher") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = username,
                onValueChange = { username = it },
                label = { Text("GitHub username") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = token,
                onValueChange = { token = it },
                label = { Text("Personal access token") },
                placeholder = { if (hasToken) Text("Saved — leave empty to keep it") },
                singleLine = true,
                visualTransformation = if (showToken) VisualTransformation.None else PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false),
                trailingIcon = {
                    IconButton(onClick = { showToken = !showToken }) {
                        Icon(
                            ScrapIcons.Visibility,
                            contentDescription = if (showToken) "Hide token" else "Show token",
                            tint = if (showToken) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                },
                supportingText = { Text("Stored encrypted on this device") },
                modifier = Modifier.fillMaxWidth(),
            )
            TextButton(onClick = { runCatching { uri.openUri(NEW_TOKEN_URL) } }) {
                Text("Create a token on GitHub")
            }
            OutlinedTextField(
                value = branch,
                onValueChange = { branch = it },
                label = { Text("Branch") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(autoCorrectEnabled = false),
                modifier = Modifier.fillMaxWidth(),
            )
            Text("Commit author (optional)", style = MaterialTheme.typography.titleSmall)
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text("Name") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = email,
                onValueChange = { email = it },
                label = { Text("Email") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email, autoCorrectEnabled = false),
                modifier = Modifier.fillMaxWidth(),
            )

            if (vm.syncing) LinearProgressIndicator(Modifier.fillMaxWidth())
            val status = vm.syncStatus
            if (!adding && status != null && current.url.isNotBlank()) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = if (status.failed) {
                            MaterialTheme.colorScheme.errorContainer
                        } else {
                            MaterialTheme.colorScheme.surfaceContainerHigh
                        },
                    ),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.padding(16.dp)) {
                        Text("Last sync", style = MaterialTheme.typography.labelLarge)
                        SyncStatusText(status)
                    }
                }
            }

            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (!adding && current.configured) {
                    OutlinedButton(onClick = { confirmDisconnect = true }) { Text("Disconnect") }
                }
                Spacer(Modifier.weight(1f))
                Button(
                    enabled = canSave,
                    onClick = {
                        val details = Repo(
                            id = "",
                            name = label.trim(),
                            url = repo!!,
                            username = username.trim(),
                            branch = branch.trim().ifEmpty { "main" },
                            authorName = name.trim(),
                            authorEmail = email.trim(),
                        )
                        if (adding) vm.addRepo(details, token) else vm.connect(details, token)
                        token = ""
                    },
                ) { Text(if (adding) "Add and sync" else "Save and sync") }
            }
        }
    }

    if (confirmDisconnect) {
        AlertDialog(
            onDismissRequest = { confirmDisconnect = false },
            title = { Text("Disconnect from GitHub?") },
            text = { Text("Your notes stay on this device. The saved token is removed and syncing stops.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDisconnect = false
                    vm.disconnect()
                    vm.back()
                }) { Text("Disconnect") }
            },
            dismissButton = { TextButton(onClick = { confirmDisconnect = false }) { Text("Cancel") } },
        )
    }
}
