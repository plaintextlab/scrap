package dev.plaintextlab.scrap.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.SearchBar
import androidx.compose.material3.SearchBarDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import dev.plaintextlab.scrap.Screen
import dev.plaintextlab.scrap.SearchHit
import dev.plaintextlab.scrap.VaultViewModel
import kotlinx.coroutines.delay

/** Full-screen Material 3 search across note names and contents. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SearchScreen(vm: VaultViewModel) {
    var query by remember { mutableStateOf("") }
    val hits by produceState(emptyList<SearchHit>(), query) {
        delay(150)
        value = vm.search(query)
    }
    SearchBar(
        inputField = {
            SearchBarDefaults.InputField(
                query = query,
                onQueryChange = { query = it },
                onSearch = {},
                expanded = true,
                onExpandedChange = { if (!it) vm.back() },
                placeholder = { Text("Search notes") },
                leadingIcon = {
                    IconButton(onClick = { vm.back() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { query = "" }) {
                            Icon(Icons.Default.Close, contentDescription = "Clear")
                        }
                    }
                },
            )
        },
        expanded = true,
        onExpandedChange = { if (!it) vm.back() },
        modifier = Modifier.fillMaxSize(),
    ) {
        LazyColumn {
            itemsIndexed(hits, key = { i, h -> "$i:${h.rel}" }) { _, hit ->
                ListItem(
                    headlineContent = { Text(hit.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    supportingContent = if (hit.text.isEmpty()) null else ({
                        Text(hit.text, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    }),
                    leadingContent = { Icon(ScrapIcons.Note, contentDescription = null) },
                    modifier = Modifier.clickable { vm.navigate(Screen.Note(hit.rel)) },
                )
            }
        }
    }
}
