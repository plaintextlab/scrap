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
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat
import dev.plaintextlab.scrap.AppSettings
import dev.plaintextlab.scrap.ThemeMode
import dev.plaintextlab.scrap.ThemeName

/** Colours of one theme in one mode. Same values as `src/theme.rs` on the desktop. */
@Immutable
data class Palette(
    val bg: Color,
    val panel: Color,
    val surface: Color,
    val border: Color,
    val text: Color,
    val muted: Color,
    val faint: Color,
    val accent: Color,
    val onAccent: Color,
    val accentSoft: Color,
    val hover: Color,
    val codeBg: Color,
    val red: Color,
    val green: Color,
    val yellow: Color,
    val blue: Color,
    val purple: Color,
    val aqua: Color,
    val orange: Color,
    val gray: Color,
)

/** The named accent colours of the current theme, for colour-coded icons and markdown. */
@Immutable
data class AccentColors(
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

private fun c(hex: Long) = Color(0xFF000000 or hex)

@Suppress("LongParameterList")
private fun p(
    bg: Long, panel: Long, surface: Long, border: Long, text: Long, muted: Long, faint: Long,
    accent: Long, onAccent: Long, accentSoft: Long, hover: Long, codeBg: Long,
    red: Long, green: Long, yellow: Long, blue: Long, purple: Long, aqua: Long, orange: Long, gray: Long,
) = Palette(
    c(bg), c(panel), c(surface), c(border), c(text), c(muted), c(faint), c(accent), c(onAccent),
    c(accentSoft), c(hover), c(codeBg), c(red), c(green), c(yellow), c(blue), c(purple), c(aqua),
    c(orange), c(gray),
)

/** (light, dark) palette of a theme. */
fun palettes(t: ThemeName): Pair<Palette, Palette> = when (t) {
    ThemeName.Gruvbox -> p(
        0xFBF1C7, 0xF2E5BC, 0xF9F5D7, 0xD5C4A1, 0x3C3836, 0x7C6F64, 0xA89984,
        0xB57614, 0xFBF1C7, 0xD5C4A1, 0xEBDBB2, 0xF2E5BC,
        0x9D0006, 0x79740E, 0xB57614, 0x076678, 0x8F3F71, 0x427B58, 0xAF3A03, 0x7C6F64,
    ) to p(
        0x282828, 0x1D2021, 0x32302F, 0x3C3836, 0xEBDBB2, 0xA89984, 0x7C6F64,
        0xFABD2F, 0x282828, 0x504945, 0x3C3836, 0x32302F,
        0xFB4934, 0xB8BB26, 0xFABD2F, 0x83A598, 0xD3869B, 0x8EC07C, 0xFE8019, 0x928374,
    )
    ThemeName.Everforest -> p(
        0xFDF6E3, 0xF4F0D9, 0xFFFBEF, 0xE0DCC7, 0x5C6A72, 0x829181, 0xA6B0A0,
        0x8DA101, 0x2D353B, 0xE6E2CC, 0xEFEBD4, 0xF4F0D9,
        0xF85552, 0x8DA101, 0xDFA000, 0x3A94C5, 0xDF69BA, 0x35A77C, 0xF57D26, 0x939F91,
    ) to p(
        0x2D353B, 0x232A2E, 0x343F44, 0x3D484D, 0xD3C6AA, 0x9DA9A0, 0x7A8478,
        0xA7C080, 0x2D353B, 0x475258, 0x3D484D, 0x343F44,
        0xE67E80, 0xA7C080, 0xDBBC7F, 0x7FBBB3, 0xD699B6, 0x83C092, 0xE69875, 0x859289,
    )
    ThemeName.Nord -> p(
        0xECEFF4, 0xE5E9F0, 0xF5F7FA, 0xD8DEE9, 0x2E3440, 0x4C566A, 0x7B88A1,
        0x5E81AC, 0xECEFF4, 0xD8DEE9, 0xDEE3EB, 0xE5E9F0,
        0xBF616A, 0x6F8F57, 0xB08A3B, 0x5E81AC, 0x9A6F94, 0x4C8C8B, 0xC0724F, 0x7B88A1,
    ) to p(
        0x2E3440, 0x292E39, 0x3B4252, 0x434C5E, 0xE5E9F0, 0xA3ACBD, 0x616E88,
        0x88C0D0, 0x2E3440, 0x434C5E, 0x3B4252, 0x3B4252,
        0xBF616A, 0xA3BE8C, 0xEBCB8B, 0x81A1C1, 0xB48EAD, 0x8FBCBB, 0xD08770, 0x616E88,
    )
    ThemeName.Catppuccin -> p(
        0xEFF1F5, 0xE6E9EF, 0xF5F6F9, 0xCCD0DA, 0x4C4F69, 0x6C6F85, 0x9CA0B0,
        0x8839EF, 0xEFF1F5, 0xCCD0DA, 0xDCE0E8, 0xE6E9EF,
        0xD20F39, 0x40A02B, 0xDF8E1D, 0x1E66F5, 0x7287FD, 0x179299, 0xFE640B, 0x8C8FA1,
    ) to p(
        0x1E1E2E, 0x181825, 0x313244, 0x45475A, 0xCDD6F4, 0xA6ADC8, 0x6C7086,
        0xCBA6F7, 0x1E1E2E, 0x45475A, 0x313244, 0x313244,
        0xF38BA8, 0xA6E3A1, 0xF9E2AF, 0x89B4FA, 0xB4BEFE, 0x94E2D5, 0xFAB387, 0x7F849C,
    )
    ThemeName.Solarized -> p(
        0xFDF6E3, 0xEEE8D5, 0xFFFBEE, 0xDDD6C1, 0x586E75, 0x657B83, 0x93A1A1,
        0x268BD2, 0xFDF6E3, 0xE1DAC3, 0xE8E2CE, 0xEEE8D5,
        0xDC322F, 0x859900, 0xB58900, 0x268BD2, 0x6C71C4, 0x2AA198, 0xCB4B16, 0x93A1A1,
    ) to p(
        0x002B36, 0x00212B, 0x073642, 0x0D4250, 0x93A1A1, 0x839496, 0x586E75,
        0x268BD2, 0xFDF6E3, 0x104858, 0x073642, 0x073642,
        0xDC322F, 0x859900, 0xB58900, 0x268BD2, 0x6C71C4, 0x2AA198, 0xCB4B16, 0x657B83,
    )
    ThemeName.TokyoNight -> p(
        0xE1E2E7, 0xD5D6DB, 0xE9E9ED, 0xC4C8DA, 0x3760BF, 0x6172B0, 0x848CB5,
        0x2E7DE9, 0xE9E9ED, 0xB7C1E3, 0xCBCFDD, 0xD0D3DD,
        0xF52A65, 0x587539, 0x8C6C3E, 0x2E7DE9, 0x9854F1, 0x118C74, 0xB15C00, 0x848CB5,
    ) to p(
        0x1A1B26, 0x16161E, 0x24283B, 0x292E42, 0xC0CAF5, 0xA9B1D6, 0x565F89,
        0x7AA2F7, 0x1A1B26, 0x283457, 0x292E42, 0x24283B,
        0xF7768E, 0x9ECE6A, 0xE0AF68, 0x7AA2F7, 0xBB9AF7, 0x73DACA, 0xFF9E64, 0x565F89,
    )
}

fun palette(t: ThemeName, dark: Boolean): Palette = palettes(t).let { if (dark) it.second else it.first }

private fun accents(p: Palette, dark: Boolean) =
    AccentColors(dark, p.red, p.green, p.yellow, p.blue, p.purple, p.aqua, p.orange, p.gray)

/** Material 3 roles filled from a theme palette. */
private fun scheme(p: Palette, dark: Boolean): ColorScheme {
    val errorContainer = lerp(p.bg, p.red, if (dark) 0.3f else 0.2f)
    return if (dark) {
        darkColorScheme(
            primary = p.accent, onPrimary = p.onAccent,
            primaryContainer = p.accent, onPrimaryContainer = p.onAccent, inversePrimary = p.accent,
            secondary = p.orange, onSecondary = p.bg,
            secondaryContainer = p.accentSoft, onSecondaryContainer = p.text,
            tertiary = p.aqua, onTertiary = p.bg,
            tertiaryContainer = p.hover, onTertiaryContainer = p.aqua,
            background = p.bg, onBackground = p.text,
            surface = p.bg, onSurface = p.text,
            surfaceVariant = p.hover, onSurfaceVariant = p.muted,
            surfaceTint = p.accent, inverseSurface = p.text, inverseOnSurface = p.bg,
            error = p.red, onError = p.bg, errorContainer = errorContainer, onErrorContainer = p.text,
            outline = p.faint, outlineVariant = p.border, scrim = Color.Black,
            surfaceBright = p.hover, surfaceDim = p.panel,
            surfaceContainerLowest = p.panel, surfaceContainerLow = p.bg, surfaceContainer = p.surface,
            surfaceContainerHigh = p.hover, surfaceContainerHighest = p.accentSoft,
        )
    } else {
        lightColorScheme(
            primary = p.accent, onPrimary = p.onAccent,
            primaryContainer = p.accent, onPrimaryContainer = p.onAccent, inversePrimary = p.accent,
            secondary = p.orange, onSecondary = p.bg,
            secondaryContainer = p.hover, onSecondaryContainer = p.text,
            tertiary = p.aqua, onTertiary = p.bg,
            tertiaryContainer = p.hover, onTertiaryContainer = p.aqua,
            background = p.bg, onBackground = p.text,
            surface = p.bg, onSurface = p.text,
            surfaceVariant = p.hover, onSurfaceVariant = p.muted,
            surfaceTint = p.accent, inverseSurface = p.text, inverseOnSurface = p.bg,
            error = p.red, onError = p.bg, errorContainer = errorContainer, onErrorContainer = p.text,
            outline = p.faint, outlineVariant = p.border, scrim = Color.Black,
            surfaceBright = p.surface, surfaceDim = p.hover,
            surfaceContainerLowest = p.surface, surfaceContainerLow = p.bg, surfaceContainer = p.panel,
            surfaceContainerHigh = p.hover, surfaceContainerHighest = p.accentSoft,
        )
    }
}

/** Accent colours of the current theme. */
val LocalAccents = staticCompositionLocalOf { accents(palette(ThemeName.Gruvbox, true), true) }

/** Material 3 components in the chosen colour theme, light or dark as the setting says. */
@Composable
fun ScrapTheme(settings: AppSettings, content: @Composable () -> Unit) {
    val dark = when (settings.theme) {
        ThemeMode.System -> isSystemInDarkTheme()
        ThemeMode.Light -> false
        ThemeMode.Dark -> true
    }
    val p = palette(settings.colorTheme, dark)

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

    CompositionLocalProvider(LocalAccents provides accents(p, dark)) {
        MaterialTheme(colorScheme = scheme(p, dark), content = content)
    }
}
