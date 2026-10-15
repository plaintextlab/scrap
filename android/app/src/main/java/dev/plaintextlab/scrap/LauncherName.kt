package dev.plaintextlab.scrap

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager

/**
 * The name under the app's icon. Android fixes an app's label at install time, so each
 * choice is an `<activity-alias>` in the manifest (with its own label) and exactly one of
 * them is enabled.
 */
object LauncherName {
    /** Must match the `.Launcher<Name>` aliases in AndroidManifest.xml. */
    val presets = listOf("Scrap", "Notes", "Notebook", "Journal", "Wiki", "Vault", "Memo")
    private const val DEFAULT = "Scrap"

    private fun component(context: Context, name: String) =
        ComponentName(context.packageName, "dev.plaintextlab.scrap.Launcher$name")

    private fun isEnabled(context: Context, name: String): Boolean =
        when (context.packageManager.getComponentEnabledSetting(component(context, name))) {
            PackageManager.COMPONENT_ENABLED_STATE_ENABLED -> true
            PackageManager.COMPONENT_ENABLED_STATE_DEFAULT -> name == DEFAULT
            else -> false
        }

    fun current(context: Context): String = presets.firstOrNull { isEnabled(context, it) } ?: DEFAULT

    /** Shows [name] in the launcher. The launcher may take a few seconds to update. */
    fun set(context: Context, name: String) {
        require(name in presets)
        val pm = context.packageManager
        // Enable the new entry before disabling the old, so there's always one launcher icon.
        pm.setComponentEnabledSetting(
            component(context, name),
            PackageManager.COMPONENT_ENABLED_STATE_ENABLED,
            PackageManager.DONT_KILL_APP,
        )
        for (other in presets) {
            if (other != name) {
                pm.setComponentEnabledSetting(
                    component(context, other),
                    PackageManager.COMPONENT_ENABLED_STATE_DISABLED,
                    PackageManager.DONT_KILL_APP,
                )
            }
        }
    }
}
