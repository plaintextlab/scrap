package dev.plaintextlab.scrap

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import dev.plaintextlab.scrap.ui.ScrapApp
import dev.plaintextlab.scrap.ui.ScrapTheme

class MainActivity : ComponentActivity() {
    private val vm: VaultViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent {
            ScrapTheme { ScrapApp(vm) }
        }
    }

    override fun onPause() {
        super.onPause()
        vm.saveNow()
    }

    override fun onRestart() {
        super.onRestart()
        vm.refreshFromDisk()
    }
}
