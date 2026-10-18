package dev.plaintextlab.scrap.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import com.mikepenz.markdown.model.DefaultMarkdownAnnotator
import com.mikepenz.markdown.model.MarkdownAnnotator
import java.util.WeakHashMap
import org.intellij.markdown.MarkdownTokenTypes
import org.intellij.markdown.ast.getTextInNode

/**
 * Coloured text, stored as `<span style="color: blue">text</span>` like the desktop app
 * (see core/src/colors.rs): markdown has no colour syntax, and this is what other editors
 * understand. The names are the theme accents, drawn in each theme's own shade.
 */
object TextColors {
    val names = listOf("red", "orange", "yellow", "green", "aqua", "blue", "purple", "gray")

    const val CLOSE = "</span>"

    fun open(name: String) = "<span style=\"color: $name\">"

    /** A `<span …>` tag that sets a colour; group 1 is the colour's value. */
    private val openTag = Regex("""<span\b[^<>\n]*?(?<![-\w])color\s*:\s*([^;"'<>]+)[^<>\n]*>""", RegexOption.IGNORE_CASE)

    /** A colour span on one line: open tag, text, close tag. */
    val span = Regex("""(<span\b[^<>\n]*>)(.*?)(</span>)""", RegexOption.IGNORE_CASE)

    /** The colour a `<span>` tag sets, if any. */
    fun valueOf(tag: String): String? = openTag.matchAt(tag, 0)?.groupValues?.get(1)?.trim()

    fun label(name: String) = name.replaceFirstChar { it.uppercase() }

    /** The theme's shade for a colour name (or a usual CSS alias), or a `#rgb`/`#rrggbb` value. */
    fun colorOf(value: String, a: AccentColors): Color? = when (value.trim().lowercase()) {
        "red", "crimson", "darkred" -> a.red
        "orange", "darkorange" -> a.orange
        "yellow", "gold" -> a.yellow
        "green", "lime", "darkgreen", "olive" -> a.green
        "aqua", "cyan", "teal", "turquoise" -> a.aqua
        "blue", "navy", "royalblue", "dodgerblue" -> a.blue
        "purple", "violet", "magenta", "fuchsia", "pink" -> a.purple
        "gray", "grey", "silver" -> a.gray
        else -> hex(value.trim())
    }

    private fun hex(v: String): Color? {
        val h = v.removePrefix("#").takeIf { v.startsWith("#") } ?: return null
        val full = when (h.length) {
            3 -> h.map { "$it$it" }.joinToString("")
            6 -> h
            else -> return null
        }
        return full.toLongOrNull(16)?.let { Color(0xFF000000 or it) }
    }

    /**
     * Colours the selection ([name]), or removes its colour (null). A selection that is
     * already coloured, just inside or including its tags, is recoloured rather than wrapped
     * again. With no selection, the coloured text around the cursor is recoloured, or else
     * the cursor's line (after any list, heading or quote marker); on an empty line an empty
     * coloured span is inserted to type into. Mirrors `format::color` in the desktop app.
     */
    fun apply(v: TextFieldValue, name: String?): TextFieldValue {
        val text = v.text
        val cursor = v.selection.collapsed
        var a = v.selection.min
        var z = v.selection.max
        if (cursor) {
            val span = enclosingSpan(text, a)
            if (span != null && span.first == span.last + 1 && name != null) {
                // An empty span next to text on the line: drop it and colour the line instead.
                val openAt = text.lastIndexOf('<', span.first - 1).takeIf { it >= 0 } ?: span.first
                val cleaned = text.substring(0, openAt) + text.substring(span.first + CLOSE.length)
                val line = lineText(cleaned, openAt)
                if (line.first < line.second) return apply(TextFieldValue(cleaned, TextRange(openAt)), name)
            }
            if (span != null) {
                a = span.first
                z = span.last + 1
            } else {
                val line = lineText(text, a)
                a = line.first
                z = line.second
            }
        }
        val selected = text.substring(a, z)
        var outerA = a
        var outerZ = z
        var innerA = a
        var innerZ = z
        val whole = openTag.matchAt(selected, 0)
        if (whole != null && selected.endsWith(CLOSE, ignoreCase = true) && whole.value.length + CLOSE.length <= selected.length) {
            innerA = a + whole.value.length
            innerZ = z - CLOSE.length
        } else {
            val at = if (a > 0) text.lastIndexOf('<', a - 1) else -1
            val tag = if (at >= 0) openTag.matchAt(text, at) else null
            if (tag != null && at + tag.value.length == a && text.startsWith(CLOSE, z, ignoreCase = true)) {
                outerA = at
                outerZ = z + CLOSE.length
            }
        }
        val inner = text.substring(innerA, innerZ)
        if (name == null && outerA == innerA && outerZ == innerZ) return v
        val open = name?.let(::open) ?: ""
        val close = if (name != null) CLOSE else ""
        val out = text.substring(0, outerA) + open + inner + close + text.substring(outerZ)
        val first = outerA + open.length
        val last = first + inner.length
        // A selection stays selected; a cursor goes to the end of the coloured text.
        return TextFieldValue(out, if (cursor) TextRange(last) else TextRange(first, last))
    }

    /** The text inside the colour span the cursor is in (or right next to), on its line. */
    private fun enclosingSpan(text: String, at: Int): IntRange? {
        val lineStart = text.lastIndexOf('\n', at - 1) + 1
        val lineEnd = text.indexOf('\n', at).let { if (it < 0) text.length else it }
        var best: IntRange? = null
        var i = text.indexOf('<', lineStart)
        while (i in lineStart until lineEnd) {
            val tag = openTag.matchAt(text, i)
            if (tag != null) {
                val inner = i + tag.value.length
                val close = text.indexOf(CLOSE, inner, ignoreCase = true)
                if (close in inner..lineEnd && i <= at && at <= close + CLOSE.length) best = inner until close
            }
            i = text.indexOf('<', i + 1)
            if (i < 0) break
        }
        return best
    }

    private val lineMarker = Regex("""^(#{1,6} |[-*] \[[ xX]] |[-*+] |> |\d{1,9}[.)] )""")

    /** The cursor's line without its marker and surrounding spaces, or an empty range at the cursor. */
    private fun lineText(text: String, at: Int): Pair<Int, Int> {
        val lineStart = text.lastIndexOf('\n', at - 1) + 1
        val lineEnd = text.indexOf('\n', at).let { if (it < 0) text.length else it }
        val line = text.substring(lineStart, lineEnd).trimEnd()
        val body = line.trimStart()
        var skip = line.length - body.length
        skip += lineMarker.find(body)?.value?.length ?: 0
        val rest = line.substring(skip)
        val a = lineStart + skip + (rest.length - rest.trimStart().length)
        val z = lineStart + line.length
        return if (a >= z) at to at else a to z
    }

    /** Colours the text between `<span style="color: …">` and `</span>` in the reader. */
    fun annotator(accents: AccentColors): MarkdownAnnotator {
        // For each open <span>, per string being built: whether it pushed a colour.
        val open = WeakHashMap<AnnotatedString.Builder, ArrayDeque<Boolean>>()
        return DefaultMarkdownAnnotator { content, child ->
            if (child.type != MarkdownTokenTypes.HTML_TAG) return@DefaultMarkdownAnnotator false
            val tag = child.getTextInNode(content).toString().trim()
            val stack = open.getOrPut(this) { ArrayDeque() }
            when {
                tag.equals(CLOSE, ignoreCase = true) -> {
                    if (stack.removeLastOrNull() == true) pop()
                    true
                }
                tag.startsWith("<span", ignoreCase = true) -> {
                    val color = valueOf(tag)?.let { colorOf(it, accents) }
                    if (color != null) pushStyle(SpanStyle(color = color))
                    stack.addLast(color != null)
                    true
                }
                else -> false
            }
        }
    }
}
