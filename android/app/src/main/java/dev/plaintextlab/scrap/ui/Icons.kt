package dev.plaintextlab.scrap.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.unit.dp

/**
 * The few Material Symbols not in the core icon set, defined here instead of pulling in
 * the multi-megabyte material-icons-extended artifact.
 */
object ScrapIcons {
    val Folder = icon(
        "Folder",
        "M10,4H4c-1.1,0 -1.99,0.9 -1.99,2L2,18c0,1.1 0.9,2 2,2h16c1.1,0 2,-0.9 2,-2V8c0,-1.1 -0.9,-2 -2,-2h-8l-2,-2z",
    )
    val Note = icon(
        "Description",
        "M14,2H6c-1.1,0 -1.99,0.9 -1.99,2L4,20c0,1.1 0.89,2 1.99,2H18c1.1,0 2,-0.9 2,-2V8l-6,-6zM16,18H8v-2h8v2z" +
            "M16,14H8v-2h8v2zM13,9V3.5L18.5,9H13z",
    )
    val NewFolder = icon(
        "CreateNewFolder",
        "M20,6h-8l-2,-2H4c-1.11,0 -1.99,0.89 -1.99,2L2,18c0,1.11 0.89,2 2,2h16c1.11,0 2,-0.89 2,-2V8" +
            "c0,-1.11 -0.89,-2 -2,-2zM19,14h-3v3h-2v-3h-3v-2h3V9h2v3h3v2z",
    )
    val Link = icon(
        "Link",
        "M3.9,12c0,-1.71 1.39,-3.1 3.1,-3.1h4V7H7c-2.76,0 -5,2.24 -5,5s2.24,5 5,5h4v-1.9H7c-1.71,0 -3.1,-1.39 -3.1,-3.1z" +
            "M8,13h8v-2H8v2zM17,7h-4v1.9h4c1.71,0 3.1,1.39 3.1,3.1s-1.39,3.1 -3.1,3.1h-4V17h4c2.76,0 5,-2.24 5,-5" +
            "s-2.24,-5 -5,-5z",
    )
    val Empty = icon(
        "EditNote",
        "M3,10h11v2H3V10zM3,8h11V6H3V8zM3,16h7v-2H3V16zM18.01,12.87l0.71,-0.71c0.39,-0.39 1.02,-0.39 1.41,0l0.71,0.71" +
            "c0.39,0.39 0.39,1.02 0,1.41l-0.71,0.71L18.01,12.87zM17.3,13.58l-5.3,5.3V21h2.12l5.3,-5.3L17.3,13.58z",
    )

    private fun icon(name: String, path: String): ImageVector =
        ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f)
            .addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black))
            .build()
}
