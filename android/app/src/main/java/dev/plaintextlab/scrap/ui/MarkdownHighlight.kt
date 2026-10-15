package dev.plaintextlab.scrap.ui

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.em

/**
 * Styles markdown in the editor without changing the text, so cursor positions map 1:1.
 */
class MarkdownHighlight(private val c: ColorScheme, private val g: GruvboxColors) : VisualTransformation {

    override fun filter(text: AnnotatedString): TransformedText =
        TransformedText(highlight(text.text), OffsetMapping.Identity)

    private val dim = SpanStyle(color = c.onSurfaceVariant.copy(alpha = 0.6f))
    private val code = SpanStyle(fontFamily = FontFamily.Monospace, color = g.green, background = c.surfaceVariant)
    private val link = SpanStyle(color = g.blue)
    private val marker = SpanStyle(color = g.orange, fontWeight = FontWeight.Bold)
    private val headingSizes = listOf(1.6f, 1.4f, 1.25f, 1.1f, 1.05f, 1f)

    private val heading = Regex("""^\s{0,3}(#{1,6})\s""")
    private val listMarker = Regex("""^\s*([-*+]( \[[ xX]\])?|\d{1,9}[.)])\s""")
    private val inline = listOf(
        Regex("""`[^`\n]+`""") to code,
        Regex("""\*\*[^*\n]+\*\*""") to SpanStyle(fontWeight = FontWeight.Bold),
        Regex("""(?<![*\w])\*[^*\s][^*\n]*?\*(?!\*)""") to SpanStyle(fontStyle = FontStyle.Italic),
        Regex("""~~[^~\n]+~~""") to SpanStyle(textDecoration = TextDecoration.LineThrough),
        Regex("""\[\[[^\]\n]+]]""") to link,
        Regex("""\[[^\]\n]+]\([^)\n]+\)""") to link,
        Regex("""(?<=^|\s)#[\p{L}\p{N}_/-]+""") to SpanStyle(color = g.aqua),
    )

    private fun highlight(s: String): AnnotatedString = buildAnnotatedString {
        append(s)
        var inFence = false
        var start = 0
        for (line in s.split('\n')) {
            val end = start + line.length
            val trimmed = line.trimStart()
            when {
                trimmed.startsWith("```") || trimmed.startsWith("~~~") -> {
                    inFence = !inFence
                    addStyle(code.merge(dim), start, end)
                }
                inFence -> addStyle(code, start, end)
                else -> {
                    val h = heading.find(line)
                    if (h != null) {
                        val level = h.groupValues[1].length
                        addStyle(
                            SpanStyle(color = g.yellow, fontWeight = FontWeight.Bold, fontSize = headingSizes[level - 1].em),
                            start,
                            end,
                        )
                        addStyle(dim, start, start + h.range.last)
                    } else if (trimmed.startsWith(">")) {
                        addStyle(SpanStyle(fontStyle = FontStyle.Italic, color = g.gray), start, end)
                    } else {
                        listMarker.find(line)?.let { addStyle(marker, start, start + it.range.last) }
                    }
                    for ((re, style) in inline) {
                        for (m in re.findAll(line)) addStyle(style, start + m.range.first, start + m.range.last + 1)
                    }
                }
            }
            start = end + 1
        }
    }
}
