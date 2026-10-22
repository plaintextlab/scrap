//! Showing a note or folder in the system file manager (Explorer, Finder, ...).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Opens the file manager at `path`. A folder is opened; a file is shown selected in its
/// folder where the platform supports that (Windows, macOS).
pub fn reveal(path: &Path) -> io::Result<()> {
    let path = native(path);
    if !path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} doesn't exist", path.display())));
    }
    spawn(&path)
}

/// Opens a file in the app the system uses for it (a PDF in the PDF viewer, …).
pub fn open_file(path: &Path) -> io::Result<()> {
    let path = native(path);
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("{} doesn't exist", path.display())));
    }
    let program = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(program).arg(&path).spawn().map(drop)
}

/// Vault paths join relative paths written with `/`; Explorer only understands `\`.
fn native(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().replace('/', "\\"))
    } else {
        path.to_path_buf()
    }
}

#[cfg(windows)]
fn spawn(path: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer.exe");
    if path.is_file() {
        // Explorer parses its own command line: "/select," has to be glued to the quoted path.
        cmd.raw_arg(format!("/select,\"{}\"", path.display()));
    } else {
        cmd.arg(path);
    }
    // Explorer's exit code is meaningless (often 1 on success), so only spawning is checked.
    cmd.spawn().map(drop)
}

#[cfg(target_os = "macos")]
fn spawn(path: &Path) -> io::Result<()> {
    let mut cmd = Command::new("open");
    if path.is_file() {
        cmd.arg("-R");
    }
    cmd.arg(path).spawn().map(drop)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn(path: &Path) -> io::Result<()> {
    // xdg-open opens the default file manager (through the portal inside a Flatpak). It
    // can't select a file, so a file's folder is opened instead.
    let dir = if path.is_file() { path.parent().unwrap_or(path) } else { path };
    Command::new("xdg-open").arg(dir).spawn().map(drop)
}
