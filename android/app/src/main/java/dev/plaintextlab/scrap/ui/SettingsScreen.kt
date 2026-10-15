package dev.plaintextlab.scrap.ui

import android.text.format.DateUtils
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Info
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Slider
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberTopAppBarState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.RadioButton
import androidx.compose.material3.TextButton
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.Role
import dev.plaintextlab.scrap.LauncherName
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.SyncStatus
import dev.plaintextlab.scrap.ThemeMode
import dev.plaintextlab.scrap.VaultViewModel
import kotlin.math.roundToInt

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(vm: VaultViewModel, snackbar: SnackbarHostState) {
    val scroll = TopAppBarDefaults.exitUntilCollapsedScrollBehavior(rememberTopAppBarState())
    val s = vm.settings
    val context = LocalContext.current
    val uri = LocalUriHandler.current
    val gruvbox = LocalGruvbox.current
    var pickingName by remember { mutableStateOf(false) }
    val version = remember(context) {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull() ?: "?"
    }

    Scaffold(
        modifier = Modifier.nestedScroll(scroll.nestedScrollConnection),
        topBar = {
            LargeTopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = { vm.back() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                scrollBehavior = scroll,
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { padding ->
        LazyColumn(contentPadding = PaddingValues(top = padding.calculateTopPadding(), bottom = 32.dp)) {
            item { SectionTitle("Appearance") }
            item {
                ListItem(
                    headlineContent = { Text("Theme") },
                    supportingContent = {
                        SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth().padding(top = 8.dp)) {
                            val modes = ThemeMode.entries
                            modes.forEachIndexed { i, mode ->
                                SegmentedButton(
                                    selected = s.theme == mode,
                                    onClick = { vm.updateSettings { it.copy(theme = mode) } },
                                    shape = SegmentedButtonDefaults.itemShape(index = i, count = modes.size),
                                ) { Text(mode.name) }
                            }
                        }
                    },
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("App name") },
                    supportingContent = { Text("Shown under the icon in your launcher: ${vm.appName}") },
                    leadingContent = { Icon(ScrapIcons.Label, contentDescription = null, tint = gruvbox.purple) },
                    trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null) },
                    modifier = Modifier.clickable { pickingName = true },
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Text size") },
                    supportingContent = {
                        Column {
                            Slider(
                                value = s.textScale,
                                onValueChange = { v ->
                                    val snapped = (v * 10).roundToInt() / 10f
                                    vm.updateSettings { it.copy(textScale = snapped) }
                                },
                                valueRange = 0.8f..1.6f,
                                steps = 7,
                            )
                            Text(
                                "${(s.textScale * 100).roundToInt()}% — The quick brown fox jumps over the lazy dog.",
                                fontSize = (16 * s.textScale).sp,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                    },
                )
            }
            item {
                SwitchItem(
                    title = "Open notes for editing",
                    summary = "Otherwise notes open in reading view",
                    checked = s.openInEdit,
                ) { on -> vm.updateSettings { it.copy(openInEdit = on) } }
            }

            item { SectionTitle("GitHub sync") }
            if (s.sync.configured) {
                item {
                    ListItem(
                        headlineContent = { Text(repoLabel(s.sync.url)) },
                        supportingContent = { SyncStatusText(vm.syncStatus) },
                        leadingContent = { Icon(ScrapIcons.Sync, contentDescription = null, tint = gruvbox.aqua) },
                        trailingContent = {
                            if (vm.syncing) {
                                CircularProgressIndicator(Modifier.size(24.dp), strokeWidth = 2.dp)
                            } else {
                                FilledTonalButton(onClick = { vm.syncNow() }) { Text("Sync now") }
                            }
                        },
                    )
                }
                item {
                    SwitchItem(
                        title = "Sync automatically",
                        summary = "When the app opens and when you leave it",
                        checked = s.sync.auto,
                    ) { on -> vm.updateSettings { it.copy(sync = it.sync.copy(auto = on)) } }
                }
                item {
                    SwitchItem(
                        title = "Sync in the background",
                        summary = "About once an hour while online, even when the app is closed",
                        checked = s.sync.background,
                    ) { on -> vm.updateSettings { it.copy(sync = it.sync.copy(background = on)) } }
                }
                item {
                    ListItem(
                        headlineContent = { Text("Connection") },
                        supportingContent = { Text("Repository, token, branch and commit author") },
                        trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null) },
                        modifier = Modifier.clickable { vm.navigate(Screen.SyncSetup) },
                    )
                }
            } else {
                item {
                    ListItem(
                        headlineContent = { Text("Connect to GitHub") },
                        supportingContent = { Text("Back up your notes and sync them with your other devices") },
                        leadingContent = { Icon(ScrapIcons.Sync, contentDescription = null, tint = gruvbox.aqua) },
                        trailingContent = { Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, null) },
                        modifier = Modifier.clickable { vm.navigate(Screen.SyncSetup) },
                    )
                }
            }

            item { SectionTitle("About") }
            item {
                ListItem(
                    headlineContent = { Text("Scrap $version") },
                    supportingContent = { Text("${vm.notes.size} notes in ${vm.vaultPath}") },
                    leadingContent = { Icon(Icons.Default.Info, contentDescription = null, tint = gruvbox.blue) },
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Source code") },
                    supportingContent = { Text("github.com/plaintextlab/scrap") },
                    modifier = Modifier.clickable { runCatching { uri.openUri("https://github.com/plaintextlab/scrap") } },
                )
            }
        }
    }

    if (pickingName) {
        AppNameDialog(
            current = vm.appName,
            onDismiss = { pickingName = false },
            onPick = {
                pickingName = false
                vm.changeAppName(it)
            },
        )
    }
}

@Composable
private fun AppNameDialog(current: String, onDismiss: () -> Unit, onPick: (String) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(ScrapIcons.Label, contentDescription = null) },
        title = { Text("App name") },
        text = {
            Column(Modifier.selectableGroup()) {
                for (name in LauncherName.presets) {
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .selectable(selected = name == current, role = Role.RadioButton) { onPick(name) }
                            .padding(vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = name == current, onClick = null)
                        Text(name, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.padding(start = 16.dp))
                    }
                }
                Text(
                    "Home screen shortcuts may need to be re-added after a change.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 8.dp),
                )
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
fun SectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 24.dp, bottom = 4.dp),
    )
}

@Composable
private fun SwitchItem(title: String, summary: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = { Text(summary) },
        trailingContent = { Switch(checked = checked, onCheckedChange = onChange) },
        modifier = Modifier.clickable { onChange(!checked) },
    )
}

@Composable
fun SyncStatusText(status: SyncStatus?) {
    if (status == null) {
        Text("Not synced yet")
        return
    }
    val time = DateUtils.getRelativeTimeSpanString(
        status.time,
        System.currentTimeMillis(),
        DateUtils.MINUTE_IN_MILLIS,
    )
    Text(
        "$time · ${status.message}",
        color = if (status.failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = 3,
    )
    Spacer(Modifier.height(2.dp))
}

fun repoLabel(url: String): String =
    url.removePrefix("https://").removePrefix("http://").removePrefix("github.com/").removeSuffix(".git")
