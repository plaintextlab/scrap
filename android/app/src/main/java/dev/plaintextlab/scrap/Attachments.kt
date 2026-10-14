package dev.plaintextlab.scrap

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import android.webkit.MimeTypeMap
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Images embedded in notes. They live in the vault's hidden `.attachments` folder, so they
 * sync with the notes but don't show up in the notes list, and notes link to them with
 * ordinary relative markdown image links.
 */
object Attachments {
    const val DIR = ".attachments"

    /** Longest side of a stored photo; keeps camera shots from bloating the repository. */
    private const val MAX_SIDE = 2048

    /** Copies [source] into the vault and returns its vault-relative path, or null on failure. */
    fun save(context: Context, vault: File, source: Uri): String? = runCatching {
        val resolver = context.contentResolver
        val mime = resolver.getType(source).orEmpty()
        val dir = File(vault, DIR).apply { mkdirs() }
        val stamp = SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(Date())

        // Re-encode stills at a sensible size (this also applies the camera's rotation).
        // GIFs keep their animation; older Android versions just get a straight copy.
        val bitmap = if (mime != "image/gif" && Build.VERSION.SDK_INT >= 28) decode(context, source) else null
        val ext = when {
            bitmap != null -> if (mime == "image/png" && bitmap.hasAlpha()) "png" else "jpg"
            else -> MimeTypeMap.getSingleton().getExtensionFromMimeType(mime) ?: "jpg"
        }
        val target = unique(dir, "IMG-$stamp", ext)
        val partial = File(dir, ".${target.name}.part")
        partial.outputStream().use { out ->
            if (bitmap != null) {
                val format = if (ext == "png") Bitmap.CompressFormat.PNG else Bitmap.CompressFormat.JPEG
                bitmap.compress(format, 85, out)
            } else {
                resolver.openInputStream(source)!!.use { it.copyTo(out) }
            }
        }
        check(partial.renameTo(target))
        "$DIR/${target.name}"
    }.getOrNull()

    private fun decode(context: Context, source: Uri): Bitmap? = runCatching {
        if (Build.VERSION.SDK_INT < 28) return null
        ImageDecoder.decodeBitmap(ImageDecoder.createSource(context.contentResolver, source)) { decoder, info, _ ->
            val w = info.size.width
            val h = info.size.height
            val scale = MAX_SIDE.toFloat() / maxOf(w, h)
            if (scale < 1f) decoder.setTargetSize((w * scale).toInt().coerceAtLeast(1), (h * scale).toInt().coerceAtLeast(1))
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
        }
    }.getOrNull()

    private fun unique(dir: File, base: String, ext: String): File {
        var file = File(dir, "$base.$ext")
        var n = 2
        while (file.exists()) file = File(dir, "$base-${n++}.$ext")
        return file
    }

    /** Link from the note at [noteRel] to the vault-relative [path]. */
    fun relativeTo(noteRel: String, path: String): String = "../".repeat(noteRel.count { it == '/' }) + path

    /** A piece of a note for the reader: markdown text, or an image on a line of its own. */
    sealed interface Block {
        data class Text(val markdown: String) : Block
        data class Image(val url: String, val alt: String) : Block
    }

    private val imageLine = Regex("""^\s*!\[([^\]]*)]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)\s*$""")

    /**
     * Splits [markdown] at lines that hold only an image, so the reader can show those at
     * full width with their real aspect ratio (the markdown renderer crops them). Lines in
     * fenced code blocks are left alone.
     */
    fun splitImages(markdown: String): List<Block> {
        val blocks = mutableListOf<Block>()
        val text = StringBuilder()
        var fence: String? = null
        fun flush() {
            if (text.isNotBlank()) blocks += Block.Text(text.toString())
            text.clear()
        }
        for (line in markdown.lines()) {
            val trimmed = line.trimStart()
            val marker = trimmed.take(3)
            if (marker == "```" || marker == "~~~") {
                fence = if (fence == null) marker else if (fence == marker) null else fence
            }
            val image = if (fence == null) imageLine.matchEntire(line) else null
            if (image != null) {
                flush()
                blocks += Block.Image(image.groupValues[2], image.groupValues[1])
            } else {
                text.append(line).append('\n')
            }
        }
        flush()
        return blocks
    }

    private val imageLink = Regex("""(!\[[^\]]*]\()([^)\s]+)([^)]*\))""")

    /**
     * Rewrites relative image links in [markdown] (written for the note at [noteRel]) to
     * file URIs the renderer can load. Web and other absolute links are left alone.
     */
    fun resolve(markdown: String, noteRel: String, vault: File): String =
        imageLink.replace(markdown) { m ->
            val (open, link, close) = m.destructured
            if (link.contains(':')) return@replace m.value
            val file = File(File(vault, parentOf(noteRel)), Uri.decode(link.removePrefix("<").removeSuffix(">")))
            open + Uri.fromFile(file.canonicalFile).toString() + close
        }
}
