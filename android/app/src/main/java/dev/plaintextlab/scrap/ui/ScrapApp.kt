package dev.plaintextlab.scrap.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.VaultViewModel

@Composable
fun ScrapApp(vm: VaultViewModel) {
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(vm.message) {
        val text = vm.message ?: return@LaunchedEffect
        snackbar.showSnackbar(text)
        vm.message = null
    }
    BackHandler(enabled = vm.stack.size > 1) { vm.back() }

    AnimatedContent(
        targetState = vm.screen,
        transitionSpec = { fadeIn(tween(180)) togetherWith fadeOut(tween(120)) },
        label = "screen",
    ) { screen ->
        when (screen) {
            is Screen.Browse -> BrowseScreen(vm, screen.folder, snackbar)
            is Screen.Note -> NoteScreen(vm, screen.rel, snackbar)
            Screen.Search -> SearchScreen(vm)
            Screen.Settings -> SettingsScreen(vm, snackbar)
            Screen.SyncSetup -> SyncSetupScreen(vm, snackbar)
        }
    }
}
