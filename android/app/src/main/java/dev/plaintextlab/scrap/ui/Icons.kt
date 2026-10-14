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

    val Sync = icon(
        "Sync",
        "M12,4V1L8,5l4,4V6c3.31,0 6,2.69 6,6 0,1.01 -0.25,1.97 -0.7,2.8l1.46,1.46C19.54,15.03 20,13.57 20,12" +
            "c0,-4.42 -3.58,-8 -8,-8zM12,18c-3.31,0 -6,-2.69 -6,-6 0,-1.01 0.25,-1.97 0.7,-2.8L5.24,7.74" +
            "C4.46,8.97 4,10.43 4,12c0,4.42 3.58,8 8,8v3l4,-4 -4,-4v3z",
    )
    val Sort = icon("Sort", "M3,18h6v-2H3v2zM3,6v2h18V6H3zM3,13h12v-2H3v2z")
    val Visibility = icon(
        "Visibility",
        "M12,4.5C7,4.5 2.73,7.61 1,12c1.73,4.39 6,7.5 11,7.5s9.27,-3.11 11,-7.5c-1.73,-4.39 -6,-7.5 -11,-7.5z" +
            "M12,17c-2.76,0 -5,-2.24 -5,-5s2.24,-5 5,-5 5,2.24 5,5 -2.24,5 -5,5zM12,9c-1.66,0 -3,1.34 -3,3" +
            "s1.34,3 3,3 3,-1.34 3,-3 -1.34,-3 -3,-3z",
    )
    val Bold = icon(
        "FormatBold",
        "M15.6,10.79c0.97,-0.67 1.65,-1.77 1.65,-2.79 0,-2.26 -1.75,-4 -4,-4H7v14h7.04c2.09,0 3.71,-1.7 3.71,-3.79" +
            " 0,-1.52 -0.86,-2.82 -2.15,-3.42zM10,6.5h3c0.83,0 1.5,0.67 1.5,1.5s-0.67,1.5 -1.5,1.5h-3v-3z" +
            "M13.5,15.5H10v-3h3.5c0.83,0 1.5,0.67 1.5,1.5s-0.67,1.5 -1.5,1.5z",
    )
    val Italic = icon("FormatItalic", "M10,4v3h2.21l-3.42,8H6v3h8v-3h-2.21l3.42,-8H18V4z")
    val Heading = icon("Title", "M5,4v3h5.5v12h3V7H19V4z")
    val BulletList = icon(
        "FormatListBulleted",
        "M4,10.5c-0.83,0 -1.5,0.67 -1.5,1.5s0.67,1.5 1.5,1.5 1.5,-0.67 1.5,-1.5 -0.67,-1.5 -1.5,-1.5z" +
            "M4,4.5c-0.83,0 -1.5,0.67 -1.5,1.5S3.17,7.5 4,7.5 5.5,6.83 5.5,6 4.83,4.5 4,4.5z" +
            "M4,16.67c-0.74,0 -1.33,0.6 -1.33,1.33s0.6,1.33 1.33,1.33 1.33,-0.6 1.33,-1.33 -0.59,-1.33 -1.33,-1.33z" +
            "M7,19h14v-2H7v2zM7,13h14v-2H7v2zM7,5v2h14V5H7z",
    )
    val Checklist = icon(
        "CheckBox",
        "M19,3H5c-1.11,0 -2,0.9 -2,2v14c0,1.1 0.89,2 2,2h14c1.11,0 2,-0.9 2,-2V5c0,-1.1 -0.89,-2 -2,-2z" +
            "M10,17l-5,-5 1.41,-1.41L10,14.17l7.59,-7.59L19,8l-9,9z",
    )
    val Code = icon(
        "Code",
        "M9.4,16.6L4.8,12l4.6,-4.6L8,6l-6,6 6,6 1.4,-1.4zM14.6,16.6l4.6,-4.6 -4.6,-4.6L16,6l6,6 -6,6 -1.4,-1.4z",
    )
    val Undo = icon(
        "Undo",
        "M12.5,8c-2.65,0 -5.05,0.99 -6.9,2.6L2,7v9h9l-3.62,-3.62c1.39,-1.16 3.16,-1.88 5.12,-1.88" +
            " 3.54,0 6.55,2.31 7.6,5.5l2.37,-0.78C21.08,11.03 17.15,8 12.5,8z",
    )
    val Redo = icon(
        "Redo",
        "M18.4,10.6C16.55,8.99 14.15,8 11.5,8c-4.65,0 -8.58,3.03 -9.96,7.22L3.9,16c1.05,-3.19 4.05,-5.5 7.6,-5.5" +
            " 1.95,0 3.73,0.72 5.12,1.88L13,16h9V7l-3.6,3.6z",
    )
    val Image = icon(
        "Image",
        "M21,19V5c0,-1.1 -0.9,-2 -2,-2H5c-1.1,0 -2,0.9 -2,2v14c0,1.1 0.9,2 2,2h14c1.1,0 2,-0.9 2,-2z" +
            "M8.5,13.5l2.5,3.01L14.5,12l4.5,6H5l3.5,-4.5z",
    )
    val PhotoLibrary = icon(
        "PhotoLibrary",
        "M22,16V4c0,-1.1 -0.9,-2 -2,-2H8c-1.1,0 -2,0.9 -2,2v12c0,1.1 0.9,2 2,2h12c1.1,0 2,-0.9 2,-2z" +
            "M11,12l2.03,2.71L16,11l4,5H8l3,-4zM2,6v14c0,1.1 0.9,2 2,2h14v-2H4V6H2z",
    )
    val Camera = icon(
        "PhotoCamera",
        "M12,12m-3.2,0a3.2,3.2 0,1 1,6.4 0a3.2,3.2 0,1 1,-6.4 0" +
            "M9,2L7.17,4H4c-1.1,0 -2,0.9 -2,2v12c0,1.1 0.9,2 2,2h16c1.1,0 2,-0.9 2,-2V6c0,-1.1 -0.9,-2 -2,-2h-3.17" +
            "L15,2H9zM12,17c-2.76,0 -5,-2.24 -5,-5s2.24,-5 5,-5 5,2.24 5,5 -2.24,5 -5,5z",
    )
    val Move = icon(
        "DriveFileMove",
        "M20,6h-8l-2,-2H4C2.9,4 2.01,4.9 2.01,6L2,18c0,1.1 0.9,2 2,2h16c1.1,0 2,-0.9 2,-2V8C22,6.9 21.1,6 20,6z" +
            "M14,18v-3h-4v-4h4V8l5,5L14,18z",
    )
    val Restore = icon(
        "RestoreFromTrash",
        "M19,4h-3.5l-1,-1h-5l-1,1H5v2h14zM6,7v12c0,1.1 0.9,2 2,2h8c1.1,0 2,-0.9 2,-2V7H6zM14,14v4h-4v-4H8l4,-4 4,4h-2z",
    )

    private fun icon(name: String, path: String): ImageVector =
        ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f)
            .addPath(pathData = addPathNodes(path), fill = SolidColor(Color.Black))
            .build()
}
