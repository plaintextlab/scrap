package dev.plaintextlab.scrap.ui

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import dev.plaintextlab.scrap.AppSettings
import dev.plaintextlab.scrap.ThemeMode

/**
 * The Gruvbox accent colours (https://github.com/morhetz/gruvbox) for the current theme:
 * the bright variants on dark backgrounds, the faded ones on light.
 */
@Immutable
data class GruvboxColors(
    val isDark: Boolean,
    val red: Color,
    val green: Color,
    val yellow: Color,
    val blue: Color,
    val purple: Color,
    val aqua: Color,
    val orange: Color,
    val gray: Color,
)

private val GruvboxDark = GruvboxColors(
    isDark = true,
    red = Color(0xFFFB4934),
    green = Color(0xFFB8BB26),
    yellow = Color(0xFFFABD2F),
    blue = Color(0xFF83A598),
    purple = Color(0xFFD3869B),
    aqua = Color(0xFF8EC07C),
    orange = Color(0xFFFE8019),
    gray = Color(0xFF928374),
)

private val GruvboxLight = GruvboxColors(
    isDark = false,
    red = Color(0xFF9D0006),
    green = Color(0xFF79740E),
    yellow = Color(0xFFB57614),
    blue = Color(0xFF076678),
    purple = Color(0xFF8F3F71),
    aqua = Color(0xFF427B58),
    orange = Color(0xFFAF3A03),
    gray = Color(0xFF7C6F64),
)

/** Gruvbox colours of the current theme, for colour-coded icons and markdown. */
val LocalGruvbox = staticCompositionLocalOf { GruvboxDark }

// Gruvbox dark backgrounds/foregrounds: bg0_h 1d2021, bg0 282828, bg0_s 32302f, bg1 3c3836,
// bg2 504945, bg3 665c54, bg4 7c6f64; fg0 fbf1c7, fg1 ebdbb2, fg3 bdae93, fg4 a89984.
private val DarkScheme: ColorScheme = darkColorScheme(
    primary = GruvboxDark.yellow,
    onPrimary = Color(0xFF282828),
    primaryContainer = GruvboxDark.yellow,
    onPrimaryContainer = Color(0xFF282828),
    inversePrimary = GruvboxLight.yellow,
    secondary = GruvboxDark.orange,
    onSecondary = Color(0xFF282828),
    secondaryContainer = Color(0xFF504945),
    onSecondaryContainer = Color(0xFFFBF1C7),
    tertiary = GruvboxDark.aqua,
    onTertiary = Color(0xFF282828),
    tertiaryContainer = Color(0xFF3C3836),
    onTertiaryContainer = GruvboxDark.aqua,
    background = Color(0xFF282828),
    onBackground = Color(0xFFEBDBB2),
    surface = Color(0xFF282828),
    onSurface = Color(0xFFEBDBB2),
    surfaceVariant = Color(0xFF3C3836),
    onSurfaceVariant = Color(0xFFBDAE93),
    surfaceTint = GruvboxDark.yellow,
    inverseSurface = Color(0xFFEBDBB2),
    inverseOnSurface = Color(0xFF282828),
    error = GruvboxDark.red,
    onError = Color(0xFF282828),
    errorContainer = Color(0xFF5C2620),
    onErrorContainer = Color(0xFFFBF1C7),
    outline = Color(0xFF7C6F64),
    outlineVariant = Color(0xFF504945),
    scrim = Color(0xFF000000),
    surfaceBright = Color(0xFF3C3836),
    surfaceDim = Color(0xFF1D2021),
    surfaceContainerLowest = Color(0xFF1D2021),
    surfaceContainerLow = Color(0xFF282828),
    surfaceContainer = Color(0xFF32302F),
    surfaceContainerHigh = Color(0xFF3C3836),
    surfaceContainerHighest = Color(0xFF504945),
)

// Gruvbox light: bg0_h f9f5d7, bg0 fbf1c7, bg0_s f2e5bc, bg1 ebdbb2, bg2 d5c4a1, bg3 bdae93,
// bg4 a89984; fg1 3c3836, fg3 665c54, fg4 7c6f64.
private val LightScheme: ColorScheme = lightColorScheme(
    primary = GruvboxLight.yellow,
    onPrimary = Color(0xFFFBF1C7),
    primaryContainer = GruvboxDark.yellow,
    onPrimaryContainer = Color(0xFF282828),
    inversePrimary = GruvboxDark.yellow,
    secondary = GruvboxLight.orange,
    onSecondary = Color(0xFFFBF1C7),
    secondaryContainer = Color(0xFFEBDBB2),
    onSecondaryContainer = Color(0xFF3C3836),
    tertiary = GruvboxLight.aqua,
    onTertiary = Color(0xFFFBF1C7),
    tertiaryContainer = Color(0xFFEBDBB2),
    onTertiaryContainer = GruvboxLight.aqua,
    background = Color(0xFFFBF1C7),
    onBackground = Color(0xFF3C3836),
    surface = Color(0xFFFBF1C7),
    onSurface = Color(0xFF3C3836),
    surfaceVariant = Color(0xFFEBDBB2),
    onSurfaceVariant = Color(0xFF665C54),
    surfaceTint = GruvboxLight.yellow,
    inverseSurface = Color(0xFF3C3836),
    inverseOnSurface = Color(0xFFFBF1C7),
    error = GruvboxLight.red,
    onError = Color(0xFFFBF1C7),
    errorContainer = Color(0xFFF5C4B8),
    onErrorContainer = Color(0xFF3C3836),
    outline = Color(0xFFA89984),
    outlineVariant = Color(0xFFD5C4A1),
    scrim = Color(0xFF000000),
    surfaceBright = Color(0xFFF9F5D7),
    surfaceDim = Color(0xFFEBDBB2),
    surfaceContainerLowest = Color(0xFFF9F5D7),
    surfaceContainerLow = Color(0xFFFBF1C7),
    surfaceContainer = Color(0xFFF2E5BC),
    surfaceContainerHigh = Color(0xFFEBDBB2),
    surfaceContainerHighest = Color(0xFFD5C4A1),
)

/** Material 3 components coloured with Gruvbox dark or light, following the theme setting. */
@Composable
fun ScrapTheme(settings: AppSettings, content: @Composable () -> Unit) {
    val dark = when (settings.theme) {
        ThemeMode.System -> isSystemInDarkTheme()
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }

    // Status/navigation bar icons must contrast with the app theme, not the system one.
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }

    CompositionLocalProvider(LocalGruvbox provides if (dark) GruvboxDark else GruvboxLight) {
        MaterialTheme(colorScheme = if (dark) DarkScheme else LightScheme, content = content)
    }
}
