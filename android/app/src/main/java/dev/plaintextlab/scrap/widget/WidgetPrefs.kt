package dev.plaintextlab.scrap.widget

import android.content.Context

/** Which note each home-screen widget shows: a repository and a note path within it. */
object WidgetPrefs {
    data class Target(val repo: String, val rel: String)

    private fun prefs(context: Context) = context.getSharedPreferences("widgets", Context.MODE_PRIVATE)

    fun get(context: Context, widgetId: Int): Target? {
        val p = prefs(context)
        val repo = p.getString("$widgetId.repo", null) ?: return null
        val rel = p.getString("$widgetId.rel", null) ?: return null
        return Target(repo, rel)
    }

    fun set(context: Context, widgetId: Int, target: Target) {
        prefs(context).edit().putString("$widgetId.repo", target.repo).putString("$widgetId.rel", target.rel).apply()
    }

    fun remove(context: Context, widgetId: Int) {
        prefs(context).edit().remove("$widgetId.repo").remove("$widgetId.rel").apply()
    }

    /** Whether any widget shows this note. */
    fun shows(context: Context, repo: String, rel: String): Boolean {
        val all = prefs(context).all
        return all.keys.filter { it.endsWith(".repo") && all[it] == repo }
            .any { all[it.removeSuffix(".repo") + ".rel"] == rel }
    }

    /** Points widgets at a note's new path after it was renamed or moved. */
    fun relocate(context: Context, repo: String, rel: String, newRel: String) {
        val p = prefs(context)
        val edit = p.edit()
        for ((key, value) in p.all) {
            if (!key.endsWith(".rel") || p.getString(key.removeSuffix(".rel") + ".repo", null) != repo) continue
            val old = value as? String ?: continue
            when {
                old == rel -> edit.putString(key, newRel)
                old.startsWith("$rel/") -> edit.putString(key, newRel + old.removePrefix(rel))
            }
        }
        edit.apply()
    }
}
