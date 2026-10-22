package dev.plaintextlab.scrap

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import dev.plaintextlab.scrap.ui.ScrapApp
import dev.plaintextlab.scrap.ui.ScrapTheme
import dev.plaintextlab.scrap.widget.NoteWidget

class MainActivity : ComponentActivity() {
    private val vm: VaultViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) handleIntent(intent)
        setContent {
            ScrapTheme(vm.settings) { ScrapApp(vm) }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    override fun onStart() {
        super.onStart()
        vm.onForeground()
    }

    override fun onStop() {
        super.onStop()
        vm.onBackground()
    }

    private fun handleIntent(intent: Intent?) {
        when (intent?.action) {
            // "Share" from another app creates a new note with the shared text.
            Intent.ACTION_SEND -> {
                if (intent.type?.startsWith("text/") != true) return
                val text = intent.getStringExtra(Intent.EXTRA_TEXT) ?: return
                vm.importShared(text, intent.getStringExtra(Intent.EXTRA_SUBJECT))
            }
            // The home-screen widget's buttons.
            NoteWidget.ACTION_OPEN -> {
                val repo = intent.getStringExtra(NoteWidget.EXTRA_REPO) ?: return
                val rel = intent.getStringExtra(NoteWidget.EXTRA_REL) ?: return
                vm.openFromWidget(repo, rel, intent.getBooleanExtra(NoteWidget.EXTRA_EDIT, false))
            }
            NoteWidget.ACTION_NEW -> vm.newNoteFromWidget()
        }
    }
}
