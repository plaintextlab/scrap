# scrap

A minimal, markdown-based notes app written in Rust. Think Obsidian without the bloat: one native ~12 MB binary that starts instantly, no Electron, no webview, no plugin system. Your notes are plain `.md` files in a folder, and sync is just git.

Runs on Windows, macOS and Linux.

![Alt text](/screenshots/cover.png?raw=true "cover")

## Features

The desktop and Android apps share the same core and the same features:

- **Vault = any folder** of markdown files. Nothing proprietary; hidden folders (`.git`, `.obsidian`, …) are ignored, so an existing Obsidian vault opens as-is.
- **Folders, nested as deep as you like.** Create, rename, and **move** notes and folders (on the desktop also by drag and drop in the sidebar). Sort by name or last edited.
- **Editor with live syntax highlighting** and a **formatting toolbar**: undo/redo, heading, bold, italic, strikethrough, lists, checklists, quotes, `[[links]]`, code. Plus a rendered **Read** view and (desktop) a **Split** view.
- **Images in notes**: pick files, drop them on the window, or (Android) take a photo. They're downscaled, stored in the vault's hidden `.attachments/` folder, synced with your notes, and shown full width in the reader.
- **Instant search** (Ctrl+K): recent notes, matching names and matching lines with the hits highlighted. Type a name that doesn't exist to create it.
- **`[[Wiki links]]`**, including `[[Note|alias]]` and `[[Note#heading]]`. Click in Read mode, or Ctrl/Cmd+click in the editor. Linking to a note that doesn't exist creates it. **Backlinks and outgoing links** panel.
- **Renaming a note updates links** to it across the vault; **moving one keeps its image links working**.
- **Trash**: deleted notes and folders can be restored to where they were (with Undo right after deleting), deleted forever, or emptied.
- **Themes:** Gruvbox (default), Everforest, Nord, Catppuccin, Solarized and Tokyo Night, each in light and dark (or following the system) with a one-click toggle. Icons are colour-coded and the app icon follows the theme. Adjustable text size; notes open in Edit, Read or the last used view.
- **Updates itself** on Windows, macOS and Linux (AppImage; .deb/.rpm open in your installer): it checks for a new release at startup (can be turned off in Settings) and installs it with one click.
- **Autosave**, back/forward history, word and character counts.
- **Git sync**: commit, pull and push in one click, automatically on a timer, when the vault opens and when the app closes. If the same note changed on two devices, **both versions are kept** (`Note (conflict).md`) instead of leaving conflict markers.

## Screenshots

![Alt text](/screenshots/1.png?raw=true "Destkop_1")
![Alt text](/screenshots/2.png?raw=true "Destkop_2")
![Alt text](/screenshots/3.png?raw=true "Destkop_3")
![Alt text](/screenshots/4.jpg?raw=true "Android_1")
![Alt text](/screenshots/5.jpg?raw=true "Android_2")
![Alt text](/screenshots/6.jpg?raw=true "Android_3")
![Alt text](/screenshots/7.jpg?raw=true "Android_4")
![Alt text](/screenshots/8.jpg?raw=true "Android_5")


## Git sync

The desktop app shells out to your installed `git`, so it uses your existing SSH keys, credential manager and config. No tokens to paste into the app.

- **New vault:** open a folder, then go to Settings → Sync → *Set up git* → *Add remote…* (for example a private GitHub repo).
- **Second machine:** on the welcome screen choose *Clone from git*.
- **Several repositories:** each repository is its own vault (folder of notes) with its own remote, for example `notes.git` and `work.git`. Add more in Settings → Repositories (*Add from git…* or *Add a folder…*) and switch between them there or from the vault menu at the top of the sidebar. On Android: Settings → Repositories → *Add repository*, and switch from the title on the notes screen. Changes in the repository you leave are synced in the background.
- **What a sync does:** `git add -A` → commit → `fetch` → merge the remote branch → `push`.
- **Conflicts:** if the same note was edited on two machines, your version stays where it is and the other one is saved next to it as `Note (conflict).md`. Nothing is lost and you never see conflict markers.

Make sure git knows who you are (`git config --global user.name` / `user.email`). Otherwise commits fail and the app shows git's error.

## Shortcuts

`Ctrl` is `Cmd` on macOS.

| Keys | Action |
| --- | --- |
| `Ctrl+K` / `Ctrl+P` / `Ctrl+O` | Search notes (`Shift+Enter` creates a note) |
| `Ctrl+N` / `Ctrl+Shift+N` | New note / new folder |
| `Ctrl+E` | Switch between Edit and Read |
| `Ctrl+B` / `Ctrl+I` | Bold / italic |
| `Ctrl+\` | Show or hide the sidebar |
| `Ctrl+S` | Save now (autosave is always on) |
| `Ctrl+Shift+S` | Git sync |
| `Ctrl+Alt+←` / `→` | Back / forward |
| `Ctrl+click` | Follow link in the editor |
| `Ctrl +` / `Ctrl -` | Zoom |

Run `scrap <folder>` to open a specific folder. `SCRAP_CONFIG_DIR` keeps settings in another folder (for a portable install).

## Download

Get the latest version from the [Releases page](https://github.com/plaintextlab/scrap/releases):

| System | Download |
| --- | --- |
| Windows (x64) | `scrap-v…-windows-x64.exe`, a single program; run it from anywhere |
| macOS (Apple Silicon and Intel) | `scrap-v…-macos-universal.dmg`; drag Scrap to Applications |
| Linux (x86_64) | `.AppImage`, `.deb`, `.rpm` or `.flatpak` |
| Android | `scrap-v…-android-arm64.apk` |

Every push to `main` is tested and published automatically as a new release (`v<major>.<minor>.<build>`; bump major/minor in `Cargo.toml`). The desktop app checks for new releases and updates itself; a Flatpak updates through your software center.

**Android:** each release also has an `android-arm64.apk`: a native Material 3 app (Jetpack Compose) on the same Rust core, with:

- GitHub sync over HTTPS with a personal access token (stored encrypted with the Android Keystore). Syncs on open, when you leave the app, on pull-to-refresh and optionally hourly in the background.
- Settings: theme (Gruvbox, Everforest, Nord, Catppuccin, Solarized, Tokyo Night; system/light/dark, plus a toggle in the top bar), with a matching launcher icon; the app name shown in the launcher (Scrap, Notes, Notebook, Journal, Wiki, Vault or Memo); text size; open notes in edit or read mode.
- The formatting toolbar sits above the keyboard, images can come from the camera, and you can "Share to Scrap" from other apps.

Notes live in `Android/data/dev.plaintextlab.scrap/files/vault`. Every release is signed with the same key (repository secrets `ANDROID_KEYSTORE_BASE64` / `ANDROID_KEYSTORE_PASSWORD`, a PKCS#12 keystore with key alias `scrap`), so a new APK installs over the old one and keeps your notes and settings. Releases fail rather than ship an APK signed with any other key.

The macOS app isn't notarized: on first launch, right-click Scrap → Open (or run `xattr -dr com.apple.quarantine /Applications/Scrap.app`). Updates installed from within the app don't need this again.

## Building

Requires a [Rust toolchain](https://rustup.rs) and `git` on the PATH.

```sh
cargo build --release
```

The binary is at `target/release/scrap` (`scrap.exe` on Windows).

On Linux you also need the usual windowing dev packages, e.g. on Debian/Ubuntu:

```sh
sudo apt install libxkbcommon-dev libgl1-mesa-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

Run the tests with `cargo test --workspace`. They include an end-to-end sync between two clones of a local bare repo.

For screenshots, `SCRAP_SCREENSHOT=out.png` makes the app set itself up as `SCRAP_DEMO` describes, save a screenshot of its window and quit; see `src/dev.rs`. For example:

```sh
SCRAP_SCREENSHOT=read.png SCRAP_DEMO="theme:dark;open:Welcome.md;view:read" scrap path/to/vault
```

### Project layout

| Path | What |
| --- | --- |
| `core/` | `scrap-core`: vault, links, rename, move, trash, search, attachments (no UI, shared by everything) |
| `src/` | Desktop app (egui) and git sync |
| `assets/fonts/` | Inter and JetBrains Mono (SIL Open Font License), embedded in the desktop app |
| `android/rust/` | JNI bridge exposing `scrap-core` to Kotlin |
| `android/app/` | Android app (Kotlin, Jetpack Compose, Material 3) |

### Building the Android app

Needs the Android SDK + NDK, JDK 17, Gradle 8.11+ and [cargo-ndk](https://github.com/bbqsrc/cargo-ndk):

```sh
rustup target add aarch64-linux-android
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release -p scrap-android
cd android && gradle assembleRelease
```
