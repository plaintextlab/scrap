package dev.plaintextlab.scrap

/**
 * Lists and checklists: continuing a list when Enter is pressed, and ticking items.
 * Mirrors core/src/lists.rs, which the desktop app uses.
 */
object Lists {
    /** What a new note made as a checklist starts with. */
    const val NEW_CHECKLIST = "1. [ ] "

    /** A list item's marker: indent, bullet or number, optional checkbox; [prefix] is its length. */
    data class Item(val indent: String, val bullet: Char?, val number: Long?, val delim: Char, val task: Boolean?, val prefix: Int) {
        /** The marker for the next item: the next number, an unticked box. */
        fun nextMarker(): String {
            val head = bullet?.toString() ?: "${(number ?: 0) + 1}$delim"
            return indent + head + (if (task != null) " [ ]" else "") + " "
        }
    }

    private val pattern = Regex("""^([ \t]*)(?:([-*+])|(\d{1,9})([.)]))(?: |$)(?:\[([ xX])](?: |$))?""")

    fun item(line: String): Item? {
        val m = pattern.find(line) ?: return null
        val g = m.groups
        return Item(
            indent = g[1]!!.value,
            bullet = g[2]?.value?.first(),
            number = g[3]?.value?.toLong(),
            delim = g[4]?.value?.first() ?: '.',
            task = g[5]?.value?.let { it != " " },
            prefix = m.value.length,
        )
    }

    /**
     * After a newline was typed at [nl] (the index of the `\n`): continues the previous
     * line's list, or ends it if that item was empty. Returns the new text and cursor.
     */
    fun continueList(text: String, nl: Int): Pair<String, Int>? {
        if (nl !in text.indices || text[nl] != '\n') return null
        val start = text.lastIndexOf('\n', nl - 1) + 1
        val line = text.substring(start, nl)
        val item = item(line) ?: return null
        if (line.substring(item.prefix).isBlank()) {
            // Enter on an empty item: drop its marker and stay on that (now empty) line.
            return text.substring(0, start) + text.substring(nl + 1) to start
        }
        val marker = item.nextMarker()
        return text.substring(0, nl + 1) + marker + text.substring(nl + 1) to nl + 1 + marker.length
    }

    /** Line indices of checklist items, in order, outside code blocks (block quotes count). */
    fun taskLines(text: String): List<Int> {
        var fence = false
        val out = mutableListOf<Int>()
        text.split('\n').forEachIndexed { i, line ->
            val t = line.trimStart()
            if (t.startsWith("```") || t.startsWith("~~~")) {
                fence = !fence
            } else if (!fence && item(unquote(line))?.task != null) {
                out += i
            }
        }
        return out
    }

    fun unquote(line: String): String {
        var l = line
        while (true) {
            val t = l.trimStart()
            if (!t.startsWith(">")) return l
            l = t.substring(1).removePrefix(" ")
        }
    }

    /** Ticks or unticks the checklist item on line [lineIndex]; null if it isn't one. */
    fun toggleLine(text: String, lineIndex: Int): String? {
        val lines = text.split('\n').toMutableList()
        val line = lines.getOrNull(lineIndex) ?: return null
        val unquoted = unquote(line)
        val item = item(unquoted) ?: return null
        val done = item.task ?: return null
        val quote = line.length - unquoted.length
        val box = quote + unquoted.substring(0, item.prefix).lastIndexOf('[')
        lines[lineIndex] = line.substring(0, box + 1) + (if (done) ' ' else 'x') + line.substring(box + 2)
        return lines.joinToString("\n")
    }
}
