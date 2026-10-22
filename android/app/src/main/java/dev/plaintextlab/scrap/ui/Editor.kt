package dev.plaintextlab.scrap.ui

import android.content.ActivityNotFoundException
import android.net.Uri
import android.os.SystemClock
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Clear
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.FileProvider
import dev.plaintextlab.scrap.Lists
import java.io.File
import kotlinx.coroutines.launch
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

/**
 * Markdown editor with a formatting toolbar that sits above the keyboard.
 * [onAddImage] stores a picked image and returns the markdown that embeds it.
 */
@Composable
fun Editor(text: String, onChange: (String) -> Unit, padding: PaddingValues, onAddImage: suspend (Uri) -> String?) {
    val colors = MaterialTheme.colorScheme
    val accents = LocalAccents.current
    val highlight = remember(colors, accents) { MarkdownHighlight(colors, accents) }
    val focus = remember { FocusRequester() }
    val history = remember { EditHistory() }
    var value by remember { mutableStateOf(TextFieldValue(text, TextRange(text.length))) }
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var imageMenu by remember { mutableStateOf(false) }
    var colorMenu by remember { mutableStateOf(false) }
    // Survives the activity being recreated while the camera app is open.
    var cameraFile by rememberSaveable { mutableStateOf<String?>(null) }

    // The note can change underneath us (sync, rename rewriting links).
    LaunchedEffect(text) {
        if (text != value.text) {
            val end = minOf(value.selection.end, text.length)
            value = TextFieldValue(text, TextRange(end))
        }
    }

    fun update(next: TextFieldValue, record: Boolean = true) {
        if (record && next.text != value.text) history.record(value)
        value = next
        if (next.text != text) onChange(next.text)
    }

    /** Typing: a newline typed in a list starts its next item (or ends it, on an empty one). */
    fun typed(next: TextFieldValue) {
        val at = value.selection.min
        val newline = next.selection.collapsed && next.selection.start == at + 1 &&
            next.text.length == value.text.length - value.selection.length + 1 &&
            next.text.getOrNull(at) == '\n'
        val continued = if (newline) Lists.continueList(next.text, at) else null
        update(continued?.let { (t, cursor) -> TextFieldValue(t, TextRange(cursor)) } ?: next)
    }

    fun insertImage(uri: Uri, cleanup: File? = null) {
        scope.launch {
            val markdown = onAddImage(uri)
            cleanup?.delete()
            if (markdown == null) {
                Toast.makeText(context, "Couldn't add that image", Toast.LENGTH_SHORT).show()
            } else {
                update(insertBlock(value, markdown))
            }
        }
    }
    val pickImage = rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri != null) insertImage(uri)
    }
    val takePhoto = rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) { taken ->
        val file = cameraFile?.let { File(it) } ?: return@rememberLauncherForActivityResult
        cameraFile = null
        if (taken && file.length() > 0) insertImage(Uri.fromFile(file), cleanup = file) else file.delete()
    }
    fun openCamera() {
        val file = File(File(context.cacheDir, "camera").apply { mkdirs() }, "photo-${System.currentTimeMillis()}.jpg")
        cameraFile = file.path
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
        try {
            takePhoto.launch(uri)
        } catch (e: ActivityNotFoundException) {
            cameraFile = null
            Toast.makeText(context, "No camera app found", Toast.LENGTH_SHORT).show()
        }
    }

    Column(
        Modifier
            .fillMaxSize()
            .padding(padding)
            .consumeWindowInsets(padding)
            .imePadding(),
    ) {
        BasicTextField(
            value = value,
            onValueChange = { typed(it) },
            textStyle = MaterialTheme.typography.bodyLarge.copy(color = colors.onSurface),
            cursorBrush = SolidColor(colors.primary),
            visualTransformation = highlight,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 20.dp, vertical = 12.dp)
                .focusRequester(focus),
        )
        Surface(tonalElevation = 3.dp, modifier = Modifier.fillMaxWidth()) {
            Column {
                HorizontalDivider(color = colors.outlineVariant)
                Row(
                    Modifier
                        .navigationBarsPadding()
                        .horizontalScroll(rememberScrollState())
                        .padding(horizontal = 4.dp),
                ) {
                    ToolButton(ScrapIcons.Undo, "Undo", history.canUndo) {
                        history.undo(value)?.let { update(it, record = false) }
                    }
                    ToolButton(ScrapIcons.Redo, "Redo", history.canRedo) {
                        history.redo(value)?.let { update(it, record = false) }
                    }
                    VerticalDivider(Modifier.padding(vertical = 12.dp).height(24.dp))
                    ToolButton(ScrapIcons.Heading, "Heading") { update(toggleLinePrefix(value, "# ")) }
                    ToolButton(ScrapIcons.Bold, "Bold") { update(wrap(value, "**", "**")) }
                    ToolButton(ScrapIcons.Italic, "Italic") { update(wrap(value, "*", "*")) }
                    ToolButton(ScrapIcons.BulletList, "List") { update(toggleLinePrefix(value, "- ")) }
                    ToolButton(ScrapIcons.Checklist, "Checklist") { update(toggleLinePrefix(value, "- [ ] ")) }
                    ToolButton(ScrapIcons.Link, "Link to note") { update(wrap(value, "[[", "]]")) }
                    ToolButton(ScrapIcons.Code, "Code") { update(wrap(value, "`", "`")) }
                    Box {
                        ToolButton(ScrapIcons.Palette, "Text colour") { colorMenu = true }
                        DropdownMenu(expanded = colorMenu, onDismissRequest = { colorMenu = false }) {
                            for (name in TextColors.names) {
                                DropdownMenuItem(
                                    text = { Text(TextColors.label(name)) },
                                    leadingIcon = {
                                        Box(
                                            Modifier
                                                .size(20.dp)
                                                .background(TextColors.colorOf(name, accents) ?: colors.onSurface, CircleShape),
                                        )
                                    },
                                    onClick = {
                                        colorMenu = false
                                        update(TextColors.apply(value, name))
                                    },
                                )
                            }
                            HorizontalDivider()
                            DropdownMenuItem(
                                text = { Text("No colour") },
                                leadingIcon = { Icon(Icons.Default.Clear, null) },
                                onClick = {
                                    colorMenu = false
                                    update(TextColors.apply(value, null))
                                },
                            )
                        }
                    }
                    Box {
                        ToolButton(ScrapIcons.Image, "Add image") { imageMenu = true }
                        DropdownMenu(expanded = imageMenu, onDismissRequest = { imageMenu = false }) {
                            DropdownMenuItem(
                                text = { Text("Choose from device") },
                                leadingIcon = { Icon(ScrapIcons.PhotoLibrary, null) },
                                onClick = {
                                    imageMenu = false
                                    pickImage.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
                                },
                            )
                            DropdownMenuItem(
                                text = { Text("Take photo") },
                                leadingIcon = { Icon(ScrapIcons.Camera, null) },
                                onClick = {
                                    imageMenu = false
                                    openCamera()
                                },
                            )
                        }
                    }
                }
            }
        }
    }

    LaunchedEffect(Unit) {
        delay(100)
        runCatching { focus.requestFocus() }
    }
}

@Composable
private fun ToolButton(icon: ImageVector, label: String, enabled: Boolean = true, onClick: () -> Unit) {
    IconButton(onClick = onClick, enabled = enabled) {
        Icon(icon, contentDescription = label)
    }
}

/** Wraps the selection (or inserts an empty pair at the cursor). */
internal fun wrap(v: TextFieldValue, before: String, after: String): TextFieldValue {
    val start = v.selection.min
    val end = v.selection.max
    val text = v.text.substring(0, start) + before + v.text.substring(start, end) + after + v.text.substring(end)
    val selection = if (start == end) {
        TextRange(start + before.length)
    } else {
        TextRange(start + before.length, end + before.length)
    }
    return TextFieldValue(text, selection)
}

/**
 * Inserts [block] as a paragraph of its own (blank lines around it, so markdown doesn't
 * fold it into a neighbouring list item or paragraph) and puts the cursor after it.
 */
internal fun insertBlock(v: TextFieldValue, block: String): TextFieldValue {
    val at = v.selection.max
    val head = v.text.substring(0, at)
    val tail = v.text.substring(at)
    val before = when {
        head.isEmpty() || head.endsWith("\n\n") -> ""
        head.endsWith("\n") -> "\n"
        else -> "\n\n"
    }
    val after = when {
        tail.startsWith("\n\n") -> ""
        tail.startsWith("\n") -> "\n"
        else -> "\n\n"
    }
    val inserted = before + block + after
    return TextFieldValue(v.text.substring(0, at) + inserted + v.text.substring(at), TextRange(at + inserted.length))
}

/** Adds or removes a prefix such as "- " at the start of the cursor's line. */
internal fun toggleLinePrefix(v: TextFieldValue, prefix: String): TextFieldValue {
    val lineStart = v.text.lastIndexOf('\n', v.selection.min - 1) + 1
    val present = v.text.startsWith(prefix, lineStart)
    val text = if (present) {
        v.text.removeRange(lineStart, lineStart + prefix.length)
    } else {
        v.text.substring(0, lineStart) + prefix + v.text.substring(lineStart)
    }
    val shift = if (present) -prefix.length else prefix.length
    val selection = TextRange(
        maxOf(lineStart, v.selection.start + shift),
        maxOf(lineStart, v.selection.end + shift),
    )
    return TextFieldValue(text, selection)
}

/** Undo history that groups keystrokes typed within a second of each other. */
private class EditHistory {
    private val undo = mutableStateListOf<TextFieldValue>()
    private val redo = mutableStateListOf<TextFieldValue>()
    private var last = 0L

    val canUndo: Boolean get() = undo.isNotEmpty()
    val canRedo: Boolean get() = redo.isNotEmpty()

    fun record(before: TextFieldValue) {
        val now = SystemClock.uptimeMillis()
        if (undo.isEmpty() || now - last > 1000) {
            undo.add(before)
            if (undo.size > 200) undo.removeAt(0)
        }
        last = now
        redo.clear()
    }

    fun undo(current: TextFieldValue): TextFieldValue? {
        val previous = undo.removeLastOrNull() ?: return null
        redo.add(current)
        last = 0
        return previous
    }

    fun redo(current: TextFieldValue): TextFieldValue? {
        val next = redo.removeLastOrNull() ?: return null
        undo.add(current)
        last = 0
        return next
    }
}
