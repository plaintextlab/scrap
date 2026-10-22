package dev.plaintextlab.scrap

/**
 * Bindings to the Rust core (android/rust). Structured results are JSON strings;
 * failures are thrown as [java.io.IOException]. Calls block, so use a background dispatcher.
 */
object Core {
    init {
        System.loadLibrary("scrap_android")
    }

    @JvmStatic external fun open(root: String): String
    @JvmStatic external fun rescan(): String
    @JvmStatic external fun list(): String
    @JvmStatic external fun read(rel: String): String
    @JvmStatic external fun save(rel: String, text: String): String
    @JvmStatic external fun createNote(folder: String, name: String): String
    @JvmStatic external fun createFolder(parent: String, name: String): String
    @JvmStatic external fun rename(rel: String, name: String): String
    @JvmStatic external fun move(rel: String, folder: String): String
    @JvmStatic external fun delete(rel: String): String
    @JvmStatic external fun trash(): String
    @JvmStatic external fun restore(name: String): String
    @JvmStatic external fun deleteForever(name: String): String
    @JvmStatic external fun emptyTrash(): String
    @JvmStatic external fun resolve(target: String): String
    @JvmStatic external fun backlinks(rel: String): String
    @JvmStatic external fun outgoing(text: String): String
    @JvmStatic external fun search(query: String): String
    @JvmStatic external fun preview(text: String): String
    @JvmStatic external fun duplicate(rel: String): String
    @JvmStatic external fun copyTo(rel: String, folder: String): String
    /** JSON: pinned, color, tags (all), frontTags (the editable ones). */
    @JvmStatic external fun noteMeta(text: String): String
    @JvmStatic external fun setPinned(text: String, pinned: String): String
    /** An empty [color] removes it. */
    @JvmStatic external fun setColor(text: String, color: String): String
    /** [tags] is comma-separated. */
    @JvmStatic external fun setTags(text: String, tags: String): String
    @JvmStatic external fun plainText(text: String): String
    @JvmStatic external fun attachmentPath(name: String): String
    @JvmStatic external fun fileLink(rel: String, path: String): String
    /** The note as plain markdown, without colour spans. */
    @JvmStatic external fun stripColors(text: String): String
    @JvmStatic external fun sync(config: String): String
}
