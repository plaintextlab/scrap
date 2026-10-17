//! Self-update from GitHub Releases.
//!
//! Releases are built by CI with `SCRAP_VERSION` set; a build without it (a local
//! `cargo run`) never updates. Downloads use the system `curl` (shipped with Windows 10+,
//! macOS and Linux) and are checked against the SHA-256 GitHub publishes for each asset.
//!
//! How an update is applied depends on how Scrap was installed:
//! - Windows portable `.exe`: the new file replaces the running one (renamed aside first).
//! - Windows installed copy: the new setup runs silently once Scrap exits, then reopens it.
//! - macOS `.app`: the bundle is replaced from the `.dmg`.
//! - Linux AppImage: the AppImage file is replaced.
//! - Linux `.deb`/`.rpm`: the package is downloaded and opened in the system installer.
//! - Flatpak and anything else: the release page is opened.

use eframe::egui;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};

const LATEST: &str = "https://api.github.com/repos/plaintextlab/scrap/releases/latest";

/// The version this binary was released as, if it's a release build.
pub fn current_version() -> Option<&'static str> {
    option_env!("SCRAP_VERSION").filter(|v| !v.is_empty())
}

#[derive(Deserialize, Clone, Debug)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize, Clone, Debug)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub page: String,
    asset: Option<ApiAsset>,
    method: Method,
}

/// How this installation can be updated.
#[derive(Clone, Debug, PartialEq)]
enum Method {
    /// Replace this file with the downloaded one (Windows exe, Linux AppImage).
    ReplaceFile(PathBuf),
    /// Run the newer Windows setup over this installation.
    Setup,
    /// Replace this `.app` bundle from the downloaded disk image.
    MacBundle(PathBuf),
    /// Download a .deb/.rpm and open it in the system installer.
    Package,
    /// Can't update in place; show the release page.
    Manual(&'static str),
}

impl Release {
    /// Whether "Update" installs it (or at least downloads it) rather than opening a web page.
    pub fn installable(&self) -> bool {
        self.asset.is_some() && !matches!(self.method, Method::Manual(_))
    }

    /// Why it can't be installed from within the app, if it can't.
    pub fn manual_reason(&self) -> Option<&'static str> {
        match self.method {
            Method::Manual(reason) => Some(reason),
            _ if self.asset.is_none() => Some("This release has no download for this system yet."),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Installing(String),
    /// Installed; restart into this command to finish.
    Ready(Restart),
    /// Opened the system installer with this message.
    Handoff(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Restart {
    pub version: String,
    program: PathBuf,
    args: Vec<String>,
}

impl Restart {
    pub fn spawn(&self) {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        let _ = cmd.spawn();
    }
}

enum Msg {
    Checked(Result<Option<Release>, String>),
    Installed(Result<Outcome, String>),
}

enum Outcome {
    Restart(Restart),
    Handoff(String),
}

pub struct Updater {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    pub state: State,
}

impl Updater {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        cleanup();
        Updater { tx, rx, state: State::Idle }
    }

    pub fn enabled() -> bool {
        current_version().is_some()
    }

    pub fn busy(&self) -> bool {
        matches!(self.state, State::Checking | State::Installing(_))
    }

    pub fn check(&mut self, ctx: &egui::Context) {
        if self.busy() || !Self::enabled() {
            return;
        }
        self.state = State::Checking;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Checked(check()));
            ctx.request_repaint();
        });
    }

    pub fn install(&mut self, ctx: &egui::Context) {
        let State::Available(release) = self.state.clone() else { return };
        self.state = State::Installing(release.version.clone());
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Installed(install(&release)));
            ctx.request_repaint();
        });
    }

    /// Applies finished background work. Returns a message worth showing, if any.
    pub fn poll(&mut self) -> Option<String> {
        let mut note = None;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Checked(Ok(Some(release))) => {
                    note = Some(format!("Scrap {} is available", release.version));
                    self.state = State::Available(release);
                }
                Msg::Checked(Ok(None)) => self.state = State::UpToDate,
                Msg::Checked(Err(e)) => self.state = State::Failed(format!("Couldn't check for updates: {e}")),
                Msg::Installed(Ok(Outcome::Restart(r))) => self.state = State::Ready(r),
                Msg::Installed(Ok(Outcome::Handoff(m))) => self.state = State::Handoff(m),
                Msg::Installed(Err(e)) => self.state = State::Failed(format!("Update failed: {e}")),
            }
        }
        note
    }
}

// ---------------------------------------------------------------------------
// Checking
// ---------------------------------------------------------------------------

fn check() -> Result<Option<Release>, String> {
    let current = current_version().ok_or("not a release build")?;
    let body = curl(&["-fsSL", "-H", "Accept: application/vnd.github+json", LATEST])?;
    let api: ApiRelease = serde_json::from_slice(&body).map_err(|e| format!("unexpected response ({e})"))?;
    let version = api.tag_name.trim_start_matches('v').to_string();
    if !newer(&version, current) {
        return Ok(None);
    }
    let method = method();
    let asset = asset_suffix(&method).and_then(|suffix| api.assets.iter().find(|a| a.name.ends_with(suffix)).cloned());
    Ok(Some(Release { version, page: api.html_url, asset, method }))
}

/// Compares dotted version numbers ("0.1.20" > "0.1.9").
pub fn newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parse(candidate) > parse(current)
}

fn method() -> Method {
    let Ok(exe) = std::env::current_exe() else { return Method::Manual("Couldn't locate the running app.") };
    if cfg!(windows) {
        // The installer leaves its uninstaller next to scrap.exe; the portable exe has none.
        return match exe.parent() {
            Some(dir) if dir.join("unins000.exe").is_file() => Method::Setup,
            _ => Method::ReplaceFile(exe),
        };
    }
    if cfg!(target_os = "macos") {
        // .../Scrap.app/Contents/MacOS/scrap
        return match exe.ancestors().nth(3) {
            Some(bundle) if bundle.extension().is_some_and(|e| e == "app") => Method::MacBundle(bundle.to_path_buf()),
            _ => Method::Manual("Scrap isn't running from an app bundle; install the .dmg to get updates."),
        };
    }
    if Path::new("/.flatpak-info").exists() {
        return Method::Manual("Scrap was installed as a Flatpak: update it with your software center or `flatpak update`.");
    }
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Method::ReplaceFile(PathBuf::from(appimage));
    }
    if Path::new("/etc/debian_version").exists() || Path::new("/etc/redhat-release").exists() || exe.starts_with("/usr") {
        return Method::Package;
    }
    Method::Manual("This copy of Scrap wasn't installed from a release package.")
}

fn asset_suffix(method: &Method) -> Option<&'static str> {
    match method {
        Method::ReplaceFile(_) if cfg!(windows) => Some("-windows-x64.exe"),
        Method::Setup => Some("-windows-x64-setup.exe"),
        Method::ReplaceFile(_) => Some("-x86_64.AppImage"),
        Method::MacBundle(_) => Some("-macos-universal.dmg"),
        Method::Package if Path::new("/etc/debian_version").exists() => Some("-amd64.deb"),
        Method::Package => Some("-x86_64.rpm"),
        Method::Manual(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Installing
// ---------------------------------------------------------------------------

fn install(release: &Release) -> Result<Outcome, String> {
    let asset = release.asset.as_ref().ok_or("no download for this system")?;
    let version = release.version.clone();
    match &release.method {
        Method::ReplaceFile(target) => {
            let staged = with_suffix(target, ".new");
            download(asset, &staged)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
            }
            replace_file(target, &staged)?;
            Ok(Outcome::Restart(Restart { version, program: target.clone(), args: restart_args() }))
        }
        Method::Setup => {
            let dir = scratch_dir()?;
            let setup = dir.join(&asset.name);
            download(asset, &setup)?;
            // Runs when Scrap exits; setup waits for it to close, installs into the same
            // place (it remembers per-user or all-users) and starts Scrap again.
            Ok(Outcome::Restart(Restart {
                version,
                program: setup,
                args: vec![
                    "/SILENT".into(),
                    "/SUPPRESSMSGBOXES".into(),
                    "/NORESTART".into(),
                    "/CLOSEAPPLICATIONS".into(),
                    "/RELAUNCH=1".into(),
                ],
            }))
        }
        Method::MacBundle(bundle) => {
            let dir = scratch_dir()?;
            let dmg = dir.join(&asset.name);
            download(asset, &dmg)?;
            replace_bundle(bundle, &dmg, &dir)?;
            let _ = std::fs::remove_dir_all(&dir);
            Ok(Outcome::Restart(Restart {
                version,
                program: PathBuf::from("open"),
                args: vec!["-n".into(), bundle.to_string_lossy().into_owned()],
            }))
        }
        Method::Package => {
            let dir = downloads_dir();
            let file = dir.join(&asset.name);
            download(asset, &file)?;
            Command::new("xdg-open").arg(&file).spawn().map_err(|e| format!("couldn't open the installer: {e}"))?;
            Ok(Outcome::Handoff(format!(
                "Downloaded {} to {}. Finish installing it in the window that opened, then restart Scrap.",
                asset.name,
                dir.display()
            )))
        }
        Method::Manual(reason) => Err(reason.to_string()),
    }
}

/// Downloads an asset and checks it against GitHub's published SHA-256.
fn download(asset: &ApiAsset, to: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(to);
    curl(&["-fsSL", "-o", &to.to_string_lossy(), &asset.browser_download_url])?;
    if let Some(expected) = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        let actual = sha256(to)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(to);
            return Err("the download didn't match its published checksum".into());
        }
    }
    Ok(())
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Swaps `staged` in for `target`. The running file is renamed aside first, which works
/// even while it's executing (Windows won't overwrite a running exe, but it can rename it).
fn replace_file(target: &Path, staged: &Path) -> Result<(), String> {
    let old = with_suffix(target, ".old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(target, &old).map_err(|e| format!("can't replace {}: {e}", target.display()))?;
    if let Err(e) = std::fs::rename(staged, target) {
        let _ = std::fs::rename(&old, target);
        return Err(format!("can't replace {}: {e}", target.display()));
    }
    Ok(())
}

fn replace_bundle(bundle: &Path, dmg: &Path, dir: &Path) -> Result<(), String> {
    let mount = dir.join("mnt");
    std::fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
    run("hdiutil", &["attach", "-nobrowse", "-readonly", "-quiet", "-mountpoint", &mount.to_string_lossy(), &dmg.to_string_lossy()])?;
    let result = (|| {
        let source = std::fs::read_dir(&mount)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "app"))
            .ok_or("the disk image has no app in it")?;
        let staged = with_suffix(bundle, ".new");
        let old = with_suffix(bundle, ".old");
        let _ = std::fs::remove_dir_all(&staged);
        let _ = std::fs::remove_dir_all(&old);
        run("ditto", &[&source.to_string_lossy(), &staged.to_string_lossy()])?;
        std::fs::rename(bundle, &old).map_err(|e| format!("can't replace {}: {e}", bundle.display()))?;
        if let Err(e) = std::fs::rename(&staged, bundle) {
            let _ = std::fs::rename(&old, bundle);
            return Err(format!("can't replace {}: {e}", bundle.display()));
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();
    let _ = run("hdiutil", &["detach", "-quiet", &mount.to_string_lossy()]);
    result
}

/// Removes what a previous update left behind (the old exe can't be deleted while running).
fn cleanup() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(with_suffix(&exe, ".old"));
    }
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let _ = std::fs::remove_file(with_suffix(Path::new(&appimage), ".old"));
    }
}

fn restart_args() -> Vec<String> {
    // Reopen the same vault if one was passed on the command line.
    std::env::args().skip(1).collect()
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn scratch_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("scrap-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn downloads_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let downloads = home.join("Downloads");
    if downloads.is_dir() {
        downloads
    } else {
        home
    }
}

fn curl(args: &[&str]) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["--connect-timeout", "15"]).args(args).stdin(Stdio::null());
    hide_console(&mut cmd);
    let out = cmd.output().map_err(|e| format!("couldn't run curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() { "network error".into() } else { err })
    }
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new(program);
    cmd.args(args).stdin(Stdio::null());
    hide_console(&mut cmd);
    let out = cmd.output().map_err(|e| format!("couldn't run {program}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(newer("0.1.20", "0.1.9"));
        assert!(newer("0.2.0", "0.1.99"));
        assert!(!newer("0.1.19", "0.1.19"));
        assert!(!newer("0.1.18", "0.1.19"));
    }

    #[test]
    fn replaces_a_file_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("scrap-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("scrap.exe");
        let staged = with_suffix(&target, ".new");
        std::fs::write(&target, "old").unwrap();
        std::fs::write(&staged, "new").unwrap();
        replace_file(&target, &staged).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(with_suffix(&target, ".old")).unwrap(), "old");
        assert!(!staged.exists());
        assert_eq!(
            sha256(&target).unwrap(),
            "11507a0e2f5e69d5dfa40a62a1bd7b6ee57e6bcd85c67c9b8431b36fff21c437"
        );
    }
}
