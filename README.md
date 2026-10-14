# scrap

A minimal, markdown-based notes app written in Rust. Think Obsidian without the bloat: one native ~6 MB binary, no Electron, no webview, no plugin system. Your notes are plain `.md` files in a folder, and sync is just git.

Runs on Windows, macOS and Linux.

## Features

- **Vault = any folder** of markdown files. Nothing proprietary; hidden folders (`.git`, `.obsidian`, …) are ignored, so an existing Obsidian vault opens as-is.
- **Editor with live syntax highlighting**, plus a rendered **Read** view and a **Split** view.
- **`[[Wiki links]]`**, including `[[Note|alias]]` and `[[Note#heading]]`. Click in Read mode, or Ctrl/Cmd+click in the editor. Linking to a note that doesn't exist creates it.
- **Backlinks and outgoing links** panel.
- **Renaming a note updates links** to it across the vault.
- **Quick open** with fuzzy matching, and **full-text search**.
- **Autosave**, plus back/forward history.
- Deleted notes go to the vault's `.trash/` folder instead of being destroyed.
- **Git sync**: commit, pull (rebase) and push in one click, automatically on a timer, when the vault opens, and when the app closes.

## Git sync

Sync shells out to your installed `git`, so it uses your existing SSH keys, credential manager and config. No tokens to paste into the app.

- **New vault:** open a folder, then go to Settings → Git sync → *Initialize repository* → *Add remote…* (for example a private GitHub repo).
- **Second machine:** on the welcome screen choose *Clone vault from git…*.
- **What a sync does:** `git add -A` → commit → `fetch` → `rebase` onto the remote → `push`.
- **Conflicts:** if the same lines were edited on two machines, the sync stops and the rebase is aborted. Nothing is lost: your local commit stays and the files on disk are untouched. Resolve it with git in a terminal, then sync again.

Make sure git knows who you are (`git config --global user.name` / `user.email`). Otherwise commits fail and the app shows git's error.

## Shortcuts

`Ctrl` is `Cmd` on macOS.

| Keys | Action |
| --- | --- |
| `Ctrl+O` / `Ctrl+P` | Quick open (`Shift+Enter` creates a note) |
| `Ctrl+N` | New note |
| `Ctrl+E` | Toggle Edit / Read |
| `Ctrl+S` | Save now (autosave is always on) |
| `Ctrl+Shift+F` | Search |
| `Ctrl+Shift+S` | Git sync |
| `Ctrl+Alt+←` / `→` | Back / forward |
| `Ctrl+click` | Follow link in the editor |
| `Ctrl +` / `Ctrl -` | Zoom |

## Download

Prebuilt binaries for Windows, macOS (Apple Silicon and Intel) and Linux are on the [Releases page](https://github.com/plaintextlab/scrap/releases). Every push to `main` is tested and published automatically as a new release (`v<major>.<minor>.<build>`; bump major/minor in `Cargo.toml`).

**Android:** each release also has an `android-arm64.apk`: a native Material 3 app (Jetpack Compose, dynamic colour) on the same Rust core. Notes live in `Android/data/dev.plaintextlab.scrap/files/vault`. Git sync isn't available on Android yet. Unless the repo has a signing key configured (`ANDROID_KEYSTORE_BASE64` / `ANDROID_KEYSTORE_PASSWORD` secrets, key alias `scrap`), uninstall the previous version before installing a new one.

The macOS build is unsigned: on first launch, right-click → Open (or run `xattr -d com.apple.quarantine scrap`).

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

### Project layout

| Path | What |
| --- | --- |
| `core/` | `scrap-core`: vault, links, rename, search (no UI, shared by everything) |
| `src/` | Desktop app (egui) and git sync |
| `android/rust/` | JNI bridge exposing `scrap-core` to Kotlin |
| `android/app/` | Android app (Kotlin, Jetpack Compose, Material 3) |

### Building the Android app

Needs the Android SDK + NDK, JDK 17, Gradle 8.11+ and [cargo-ndk](https://github.com/bbqsrc/cargo-ndk):

```sh
rustup target add aarch64-linux-android
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release -p scrap-android
cd android && gradle assembleRelease
```
