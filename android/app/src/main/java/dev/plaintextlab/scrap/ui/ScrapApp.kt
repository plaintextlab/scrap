package dev.plaintextlab.scrap.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
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

    Box(Modifier.fillMaxSize()) {
        AnimatedContent(
            targetState = vm.screen,
            transitionSpec = { fadeIn(tween(180)) togetherWith fadeOut(tween(120)) },
            label = "screen",
        ) { screen ->
            when (screen) {
                is Screen.Browse -> BrowseScreen(vm, screen.folder, snackbar)
                is Screen.Note -> NoteScreen(vm, screen.rel, snackbar)
                Screen.Trash -> TrashScreen(vm, snackbar)
                Screen.Settings -> SettingsScreen(vm, snackbar)
                is Screen.SyncSetup -> SyncSetupScreen(vm, snackbar, screen.adding)
            }
        }
        // Search sits over the notes list; it stays open behind a result opened from it.
        AnimatedVisibility(
            visible = vm.searchOpen && vm.screen is Screen.Browse,
            enter = fadeIn(tween(150)) + slideInVertically(tween(200)) { -it / 16 },
            exit = fadeOut(tween(120)),
        ) {
            SearchOverlay(vm)
        }
    }
}
