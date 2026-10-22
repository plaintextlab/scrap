package dev.plaintextlab.scrap.widget

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.action.Action
import androidx.glance.action.ActionParameters
import androidx.glance.action.actionParametersOf
import androidx.glance.action.clickable
import androidx.glance.appwidget.CheckBox
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.material3.ColorProviders
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import dev.plaintextlab.scrap.Lists
import dev.plaintextlab.scrap.MainActivity
import dev.plaintextlab.scrap.R
import dev.plaintextlab.scrap.SettingsStore
import dev.plaintextlab.scrap.Sync
import dev.plaintextlab.scrap.ThemeMode
import dev.plaintextlab.scrap.Vault
import dev.plaintextlab.scrap.ui.palette
import dev.plaintextlab.scrap.ui.scheme
import dev.plaintextlab.scrap.titleOf
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * A note on the home screen, such as a shopping list. Checklist items can be ticked right
 * there; ✏ opens the note for editing and ＋ starts a new note in the app.
 */
class NoteWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val widgetId = GlanceAppWidgetManager(context).getAppWidgetId(id)
        provideContent {
            // Read on every composition, so an update after a save or sync shows the new text.
            val target = WidgetPrefs.get(context, widgetId)
            val text = target?.let { readNote(context, it) }
            GlanceTheme(colors = themeColors(context)) { Content(context, widgetId, target, text) }
        }
    }

    companion object {
        /** Redraws every note widget (after a note was saved, synced or ticked). */
        suspend fun refresh(context: Context) {
            runCatching { NoteWidget().updateAll(context) }
        }

        fun file(context: Context, target: WidgetPrefs.Target) = File(Vault.dir(context, target.repo), target.rel)

        fun readNote(context: Context, target: WidgetPrefs.Target): String? =
            runCatching { file(context, target).readText().replace("\r\n", "\n") }.getOrNull()

        const val ACTION_OPEN = "dev.plaintextlab.scrap.OPEN_NOTE"
        const val ACTION_NEW = "dev.plaintextlab.scrap.NEW_NOTE"
        const val EXTRA_REPO = "repo"
        const val EXTRA_REL = "rel"
        const val EXTRA_EDIT = "edit"

        /** Opens the app; each widget button gets its own URI so their PendingIntents stay apart. */
        fun appIntent(context: Context, action: String, widgetId: Int, target: WidgetPrefs.Target?, edit: Boolean = false) =
            Intent(context, MainActivity::class.java)
                .setAction(action)
                .setData(Uri.parse("scrap-widget://$widgetId/$action/$edit"))
                .putExtra(EXTRA_REPO, target?.repo)
                .putExtra(EXTRA_REL, target?.rel)
                .putExtra(EXTRA_EDIT, edit)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
    }
}

/** The app's colour theme, light and dark as its setting says. */
private fun themeColors(context: Context) = SettingsStore(context).load().let { settings ->
    val light = scheme(palette(settings.colorTheme, false), false)
    val dark = scheme(palette(settings.colorTheme, true), true)
    when (settings.theme) {
        ThemeMode.System -> ColorProviders(light = light, dark = dark)
        ThemeMode.Light -> ColorProviders(light)
        ThemeMode.Dark -> ColorProviders(dark)
    }
}

class NoteWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget = NoteWidget()

    override fun onDeleted(context: Context, appWidgetIds: IntArray) {
        super.onDeleted(context, appWidgetIds)
        appWidgetIds.forEach { WidgetPrefs.remove(context, it) }
    }
}

private val LineKey = ActionParameters.Key<Int>("line")

/** Ticks or unticks a checklist item, then redraws (and syncs if sync is automatic). */
class ToggleTaskAction : ActionCallback {
    override suspend fun onAction(context: Context, glanceId: GlanceId, parameters: ActionParameters) {
        val widgetId = GlanceAppWidgetManager(context).getAppWidgetId(glanceId)
        val target = WidgetPrefs.get(context, widgetId) ?: return
        val line = parameters[LineKey] ?: return
        val changed = withContext(Dispatchers.IO) {
            val file = NoteWidget.file(context, target)
            val text = NoteWidget.readNote(context, target) ?: return@withContext false
            val toggled = Lists.toggleLine(text, line) ?: return@withContext false
            file.writeText(toggled)
            true
        }
        if (!changed) return
        NoteWidget.refresh(context)
        val store = SettingsStore(context)
        val sync = store.load().sync
        if (target.repo == store.activeRepoId() && sync.configured && sync.auto) Sync.soon(context)
    }
}

/** A note line as the widget shows it. */
private sealed interface Shown {
    data class Task(val line: Int, val done: Boolean, val text: String) : Shown
    data class Heading(val text: String) : Shown
    data class Plain(val text: String) : Shown
}

private fun shownLines(text: String): List<Shown> {
    val out = mutableListOf<Shown>()
    var fence = false
    text.split('\n').forEachIndexed { i, raw ->
        val t = raw.trimStart()
        if (t.startsWith("```") || t.startsWith("~~~")) {
            fence = !fence
            return@forEachIndexed
        }
        if (raw.isBlank()) return@forEachIndexed
        if (fence) {
            out += Shown.Plain(raw)
            return@forEachIndexed
        }
        val line = Lists.unquote(raw)
        val item = Lists.item(line)
        val heading = Regex("""^\s{0,3}#{1,6}\s+""").find(line)
        out += when {
            item?.task != null -> Shown.Task(i, item.task, plain(line.substring(item.prefix)))
            item != null -> {
                val marker = item.bullet?.let { "•" } ?: "${item.number}${item.delim}"
                Shown.Plain(item.indent + marker + " " + plain(line.substring(item.prefix)))
            }
            heading != null -> Shown.Heading(plain(line.substring(heading.value.length)))
            else -> Shown.Plain(plain(line))
        }
        if (out.size >= 200) return out
    }
    return out
}

/** Markdown reduced to its text: no formatting marks, colour tags or link targets. */
private fun plain(s: String): String = s
    .replace(Regex("""!\[([^\]]*)]\([^)]*\)"""), "🖼 $1")
    .replace(Regex("""\[\[([^\]|]+)\|([^\]]+)]]"""), "$2")
    .replace(Regex("""\[\[([^\]]+)]]"""), "$1")
    .replace(Regex("""\[([^\]]+)]\([^)]*\)"""), "$1")
    .replace(Regex("""</?span\b[^>]*>""", RegexOption.IGNORE_CASE), "")
    .replace(Regex("""(\*\*|__|~~|`)"""), "")
    .trim()

@Composable
private fun Content(context: Context, widgetId: Int, target: WidgetPrefs.Target?, text: String?) {
    val colors = GlanceTheme.colors
    Column(
        GlanceModifier
            .fillMaxSize()
            .background(colors.background)
            .cornerRadius(20.dp)
            .padding(horizontal = 14.dp, vertical = 10.dp),
    ) {
        Row(GlanceModifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = target?.let { titleOf(it.rel) } ?: "Scrap",
                style = TextStyle(color = colors.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Bold),
                maxLines = 1,
                modifier = GlanceModifier
                    .defaultWeight()
                    .clickable(actionStartActivity(NoteWidget.appIntent(context, NoteWidget.ACTION_OPEN, widgetId, target))),
            )
            if (target != null) {
                HeaderButton(
                    R.drawable.ic_widget_edit,
                    "Edit note",
                    actionStartActivity(NoteWidget.appIntent(context, NoteWidget.ACTION_OPEN, widgetId, target, edit = true)),
                )
                Spacer(GlanceModifier.width(4.dp))
            }
            HeaderButton(
                R.drawable.ic_widget_add,
                "New note",
                actionStartActivity(NoteWidget.appIntent(context, NoteWidget.ACTION_NEW, widgetId, target)),
            )
        }
        Spacer(GlanceModifier.height(6.dp))
        val shown = text?.let(::shownLines)
        when {
            target == null -> Message("Long-press the widget to choose a note.")
            text == null -> Message("This note was moved or deleted. Long-press the widget to choose another.")
            shown.isNullOrEmpty() -> Message("This note is empty. Tap ✏ to write in it.")
            else -> LazyColumn(GlanceModifier.fillMaxSize()) {
                items(shown.orEmpty()) { line -> Line(line) }
            }
        }
    }
}

@Composable
private fun HeaderButton(icon: Int, label: String, action: Action) {
    Box(
        GlanceModifier.size(36.dp).cornerRadius(18.dp).clickable(action),
        contentAlignment = Alignment.Center,
    ) {
        Image(
            provider = ImageProvider(icon),
            contentDescription = label,
            colorFilter = ColorFilter.tint(GlanceTheme.colors.onSurfaceVariant),
            modifier = GlanceModifier.size(22.dp),
        )
    }
}

@Composable
private fun Message(text: String) {
    Text(text, style = TextStyle(color = GlanceTheme.colors.onSurfaceVariant, fontSize = 14.sp))
}

@Composable
private fun Line(line: Shown) {
    val colors = GlanceTheme.colors
    when (line) {
        is Shown.Task -> CheckBox(
            checked = line.done,
            onCheckedChange = actionRunCallback<ToggleTaskAction>(actionParametersOf(LineKey to line.line)),
            text = line.text,
            style = TextStyle(
                color = if (line.done) colors.onSurfaceVariant else colors.onSurface,
                fontSize = 15.sp,
            ),
            modifier = GlanceModifier.fillMaxWidth(),
        )
        is Shown.Heading -> Text(
            line.text,
            style = TextStyle(color = colors.primary, fontSize = 15.sp, fontWeight = FontWeight.Bold),
            modifier = GlanceModifier.padding(top = 6.dp, bottom = 2.dp),
        )
        is Shown.Plain -> Text(
            line.text,
            style = TextStyle(color = colors.onSurface, fontSize = 14.sp),
            modifier = GlanceModifier.padding(vertical = 3.dp),
        )
    }
}
