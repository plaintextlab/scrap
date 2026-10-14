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
    @JvmStatic external fun sync(config: String): String
}
