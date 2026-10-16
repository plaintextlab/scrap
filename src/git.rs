//! Git sync by shelling out to the system `git`, so the user's existing SSH keys,
//! credential helpers and config just work. Every operation runs on a worker thread.

use eframe::egui;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Default)]
pub struct Status {
    pub is_repo: bool,
    pub branch: String,
    pub remote: Option<String>,
    pub changes: usize,
    pub ahead: u32,
    pub behind: u32,
}

pub struct Finished {
    pub op: &'static str,
    pub result: Result<String, String>,
}

enum Msg {
    Status(PathBuf, Status),
    Done(PathBuf, Finished, Status),
}

pub struct Git {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    root: Option<PathBuf>,
    pub status: Option<Status>,
    pub busy: Option<&'static str>,
    pub available: bool,
}

impl Git {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Git {
            tx,
            rx,
            root: None,
            status: None,
            busy: None,
            available: git(None, &["--version"]).is_ok(),
        }
    }

    pub fn set_root(&mut self, root: Option<PathBuf>, ctx: &egui::Context) {
        self.root = root;
        self.status = None;
        self.refresh(ctx);
    }

    pub fn refresh(&self, ctx: &egui::Context) {
        let Some(root) = self.root.clone() else { return };
        if !self.available {
            return;
        }
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let st = status(&root);
            let _ = tx.send(Msg::Status(root, st));
            ctx.request_repaint();
        });
    }

    /// Collects finished operations and applies status updates for the current vault.
    pub fn poll(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Status(root, st) => {
                    if self.root.as_ref() == Some(&root) {
                        self.status = Some(st);
                    }
                }
                Msg::Done(root, finished, st) => {
                    self.busy = None;
                    if self.root.as_ref() == Some(&root) {
                        self.status = Some(st);
                    }
                    done.push(finished);
                }
            }
        }
        done
    }

    pub fn sync(&mut self, ctx: &egui::Context) {
        if let Some(root) = self.root.clone() {
            self.spawn("sync", root, ctx, sync);
        }
    }

    pub fn init(&mut self, ctx: &egui::Context) {
        if let Some(root) = self.root.clone() {
            self.spawn("init", root, ctx, init);
        }
    }

    pub fn set_remote(&mut self, url: String, ctx: &egui::Context) {
        if let Some(root) = self.root.clone() {
            self.spawn("remote", root, ctx, move |root| set_remote(root, &url));
        }
    }

    pub fn clone_repo(&mut self, url: String, dest: PathBuf, ctx: &egui::Context) {
        let target = dest.clone();
        self.spawn("clone", dest, ctx, move |_| clone(&url, &target));
    }

    fn spawn(
        &mut self,
        op: &'static str,
        root: PathBuf,
        ctx: &egui::Context,
        f: impl FnOnce(&Path) -> Result<String, String> + Send + 'static,
    ) {
        if self.busy.is_some() || !self.available {
            return;
        }
        self.busy = Some(op);
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = f(&root);
            let st = status(&root);
            let _ = tx.send(Msg::Done(root, Finished { op, result }, st));
            ctx.request_repaint();
        });
    }
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    git_bytes(dir, args).map(|out| String::from_utf8_lossy(&out).trim_end().to_string())
}

/// Inside the Flatpak sandbox there is no git, so the host's git is run instead. It sees
/// the same home folder, SSH keys and credential helpers as everywhere else.
fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// Runs git and returns its raw standard output.
fn git_bytes(dir: Option<&Path>, args: &[&str]) -> Result<Vec<u8>, String> {
    // Fail instead of hanging on a terminal prompt nobody can see.
    let env = [("GIT_TERMINAL_PROMPT", "0"), ("GIT_EDITOR", "true")];
    let mut cmd = if in_flatpak() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host");
        for (k, v) in env {
            c.arg(format!("--env={k}={v}"));
        }
        c.arg("git");
        c
    } else {
        let mut c = Command::new("git");
        c.envs(env);
        c
    };
    cmd.args(["-c", "core.quotepath=false"]);
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    cmd.args(args).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| format!("Could not run git: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        let stdout = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("git {} failed", args.join(" "))
        })
    }
}

fn run(root: &Path, args: &[&str]) -> Result<String, String> {
    git(Some(root), args)
}

pub fn status(root: &Path) -> Status {
    let mut st = Status::default();
    let Ok(out) = run(root, &["status", "--porcelain=v1", "--branch", "--", "."]) else {
        return st;
    };
    st.is_repo = true;
    for line in out.lines() {
        if let Some(head) = line.strip_prefix("## ") {
            let (name, tracking) = head.split_once(" [").unwrap_or((head, ""));
            let name = name.strip_prefix("No commits yet on ").unwrap_or(name);
            st.branch = name.split("...").next().unwrap_or(name).to_string();
            for part in tracking.trim_end_matches(']').split(", ") {
                if let Some(n) = part.strip_prefix("ahead ") {
                    st.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("behind ") {
                    st.behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.is_empty() {
            st.changes += 1;
        }
    }
    st.remote = run(root, &["remote", "get-url", "origin"]).ok().filter(|u| !u.is_empty());
    st
}

/// Commit everything, merge the remote branch, push. When the same file changed on both
/// sides, both versions are kept (like the Android app): ours stays at the original path
/// and theirs is saved next to it as "Note (conflict).md".
pub fn sync(root: &Path) -> Result<String, String> {
    let mut done = Vec::new();

    run(root, &["add", "-A", "--", "."])?;
    let staged = run(root, &["diff", "--cached", "--name-only"])?;
    let files: Vec<&str> = staged.lines().filter(|l| !l.is_empty()).collect();
    if !files.is_empty() {
        run(root, &["commit", "-q", "-m", &commit_message(&files)])?;
        done.push(format!("committed {} file{}", files.len(), if files.len() == 1 { "" } else { "s" }));
    }

    let has_remote = run(root, &["remote"]).is_ok_and(|r| r.lines().any(|l| l.trim() == "origin"));
    if !has_remote {
        done.push("no remote configured".into());
        return Ok(summary(done));
    }

    let branch = run(root, &["symbolic-ref", "--short", "HEAD"])?;
    let upstream = format!("origin/{branch}");
    let offline = |e: String| {
        if files.is_empty() {
            format!("Could not reach the remote:\n{e}")
        } else {
            format!("Committed locally, but could not reach the remote:\n{e}")
        }
    };
    run(root, &["fetch", "-q", "origin"]).map_err(offline)?;

    let remote_branch = format!("refs/remotes/{upstream}");
    let remote_has_branch = run(root, &["rev-parse", "--verify", "-q", &remote_branch]).is_ok();
    let has_commits = || run(root, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok();

    if remote_has_branch {
        if !has_commits() {
            // Empty local repo: just take what the remote has.
            run(root, &["merge", "-q", "--ff-only", &upstream])?;
            done.push("pulled".into());
        } else {
            let behind = count(root, &format!("HEAD..{upstream}"));
            if behind > 0 {
                let kept = merge(root, &upstream)?;
                if kept > 0 {
                    done.push(format!("kept both versions of {kept} conflicting note{}", plural(kept)));
                }
                done.push(format!("pulled {behind} commit{}", plural(behind as usize)));
            }
        }
    }

    if has_commits() {
        let ahead = if remote_has_branch { count(root, &format!("{upstream}..HEAD")) } else { 1 };
        if ahead > 0 {
            run(root, &["push", "-q", "-u", "origin", &branch]).map_err(offline)?;
            done.push("pushed".into());
        }
    }
    Ok(summary(done))
}

/// Merges `upstream` into HEAD, resolving conflicts by keeping both versions.
/// Returns how many files had conflicts.
fn merge(root: &Path, upstream: &str) -> Result<usize, String> {
    let Err(merge_error) = run(root, &["merge", "-q", "--no-edit", "-m", "Merge remote changes", upstream]) else {
        return Ok(0);
    };
    // No merge in progress means it failed before getting to conflicts.
    if run(root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_err() {
        return Err(format!("Couldn't merge the remote changes:\n{merge_error}"));
    }
    let resolved = resolve_conflicts(root);
    let unmerged = run(root, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
    match resolved {
        Ok(kept) if unmerged.trim().is_empty() => {
            let msg = format!("Merge remote changes (kept both versions of {kept} note{})", plural(kept));
            run(root, &["commit", "-q", "--no-edit", "-m", &msg])?;
            Ok(kept)
        }
        other => {
            let _ = run(root, &["merge", "--abort"]);
            let why = other.err().unwrap_or_else(|| "some files could not be merged".into());
            Err(format!(
                "Sync stopped: the remote changes couldn't be merged. Nothing was lost; your changes are \
                 committed locally.\n\n{why}"
            ))
        }
    }
}

fn resolve_conflicts(root: &Path) -> Result<usize, String> {
    // "<mode> <object> <stage>\t<path>" for each unmerged path and stage.
    let listing = run(root, &["ls-files", "-u", "-z"])?;
    let mut stages: std::collections::BTreeMap<String, Vec<u8>> = Default::default();
    for entry in listing.split('\0').filter(|e| !e.is_empty()) {
        let Some((meta, path)) = entry.split_once('\t') else { continue };
        let stage = meta.split_whitespace().nth(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        stages.entry(path.to_string()).or_default().push(stage);
    }
    let mut kept = 0;
    for (path, st) in stages {
        let (ours, theirs) = (st.contains(&2), st.contains(&3));
        if ours && theirs {
            let copy = free_name(&path, "conflict", |p| root.join(p).exists());
            let their_text = git_bytes(Some(root), &["show", &format!(":3:{path}")])?;
            std::fs::write(root.join(&copy), their_text).map_err(|e| e.to_string())?;
            run(root, &["checkout", "--ours", "--", &path])?;
            run(root, &["add", "--", &path, &copy])?;
            kept += 1;
        } else if ours {
            // Edited here, deleted there: keep the edit.
            run(root, &["checkout", "--ours", "--", &path])?;
            run(root, &["add", "--", &path])?;
        } else if theirs {
            // Deleted here, edited there: keep the edit.
            run(root, &["checkout", "--theirs", "--", &path])?;
            run(root, &["add", "--", &path])?;
        } else {
            // Deleted on both sides.
            run(root, &["rm", "-q", "--cached", "--ignore-unmatch", "--", &path])?;
        }
    }
    Ok(kept)
}

/// "notes/Idea.md" -> "notes/Idea (tag).md", numbered if that is taken.
fn free_name(rel: &str, tag: &str, taken: impl Fn(&str) -> bool) -> String {
    let (stem, ext) = match rel.rsplit_once('.') {
        Some((s, e)) if !e.contains('/') => (s, format!(".{e}")),
        _ => (rel, String::new()),
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 { format!("{stem} ({tag}){ext}") } else { format!("{stem} ({tag} {n}){ext}") };
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn init(root: &Path) -> Result<String, String> {
    run(root, &["init", "-q"])?;
    // Keep notes byte-identical across operating systems.
    run(root, &["config", "core.autocrlf", "false"])?;
    let ignore = root.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, format!("{}/\n", crate::vault::TRASH)).map_err(|e| e.to_string())?;
    }
    sync(root)?;
    Ok("Initialized git repository".into())
}

fn set_remote(root: &Path, url: &str) -> Result<String, String> {
    if run(root, &["remote", "get-url", "origin"]).is_ok() {
        run(root, &["remote", "set-url", "origin", url])?;
    } else {
        run(root, &["remote", "add", "origin", url])?;
    }
    Ok(format!("Remote set to {url}"))
}

fn clone(url: &str, dest: &Path) -> Result<String, String> {
    let dest_str = dest.to_string_lossy();
    git(None, &["clone", "-q", "-c", "core.autocrlf=false", url, &dest_str])?;
    Ok(dest_str.into_owned())
}

fn count(root: &Path, range: &str) -> u32 {
    run(root, &["rev-list", "--count", range]).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn commit_message(files: &[&str]) -> String {
    let names: Vec<&str> = files.iter().take(3).map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
    let mut msg = format!("Update {}", names.join(", "));
    if files.len() > 3 {
        msg += &format!(" and {} more", files.len() - 3);
    }
    let host = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    if !host.is_empty() {
        msg += &format!(" (from {host})");
    }
    msg
}

fn summary(done: Vec<String>) -> String {
    if done.is_empty() {
        "Already up to date".into()
    } else {
        let s = done.join(", ");
        let mut c = s.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    }
}

/// Folder name `git clone` would pick for a URL.
pub fn repo_name(url: &str) -> String {
    let last = url.trim_end_matches('/').rsplit(['/', ':']).next().unwrap_or("vault");
    let name = last.strip_suffix(".git").unwrap_or(last);
    if name.is_empty() { "vault".into() } else { name.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrap-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn configure(dir: &Path) {
        run(dir, &["config", "user.name", "Test"]).unwrap();
        run(dir, &["config", "user.email", "test@example.com"]).unwrap();
    }

    #[test]
    fn two_machines_sync_through_a_remote() {
        let tmp = temp_dir("git");
        let bare = tmp.join("remote.git");
        let (a, b) = (tmp.join("a"), tmp.join("b"));
        fs::create_dir_all(&bare).unwrap();
        fs::create_dir_all(&a).unwrap();
        run(&bare, &["init", "-q", "--bare"]).unwrap();

        // Machine A: new vault, init, add remote, first sync pushes.
        run(&a, &["init", "-q"]).unwrap();
        configure(&a);
        fs::write(a.join("one.md"), "hello\n").unwrap();
        init(&a).unwrap();
        assert!(fs::read_to_string(a.join(".gitignore")).unwrap().contains(".trash/"));
        set_remote(&a, &bare.to_string_lossy()).unwrap();
        let msg = sync(&a).unwrap();
        assert!(msg.contains("ushed"), "{msg}");

        // Machine B: clone.
        clone(&bare.to_string_lossy(), &b).unwrap();
        configure(&b);
        assert_eq!(fs::read_to_string(b.join("one.md")).unwrap(), "hello\n");

        // Independent edits on both sides merge cleanly.
        fs::write(b.join("two.md"), "from b\n").unwrap();
        sync(&b).unwrap();
        fs::write(a.join("three.md"), "from a\n").unwrap();
        let msg = sync(&a).unwrap();
        assert!(msg.contains("pulled") && msg.contains("pushed"), "{msg}");
        assert!(a.join("two.md").exists());
        sync(&b).unwrap();
        assert!(b.join("three.md").exists());
        let st = status(&b);
        assert!(st.is_repo && st.changes == 0 && st.ahead == 0 && st.remote.is_some());

        // Same line edited on both sides: both versions survive on both machines.
        fs::write(a.join("one.md"), "edited on a\n").unwrap();
        sync(&a).unwrap();
        fs::write(b.join("one.md"), "edited on b\n").unwrap();
        let msg = sync(&b).unwrap();
        assert!(msg.contains("kept both versions of 1"), "{msg}");
        assert_eq!(fs::read_to_string(b.join("one.md")).unwrap(), "edited on b\n");
        assert_eq!(fs::read_to_string(b.join("one (conflict).md")).unwrap(), "edited on a\n");
        assert!(!b.join(".git/MERGE_HEAD").exists());
        sync(&a).unwrap();
        assert_eq!(fs::read_to_string(a.join("one.md")).unwrap(), "edited on b\n");
        assert_eq!(fs::read_to_string(a.join("one (conflict).md")).unwrap(), "edited on a\n");

        // Deleted on one side, edited on the other: the edit is kept.
        fs::remove_file(a.join("two.md")).unwrap();
        sync(&a).unwrap();
        fs::write(b.join("two.md"), "still needed\n").unwrap();
        sync(&b).unwrap();
        assert_eq!(fs::read_to_string(b.join("two.md")).unwrap(), "still needed\n");
        let st = status(&b);
        assert_eq!((st.changes, st.ahead, st.behind), (0, 0, 0));
    }

    #[test]
    fn sync_without_remote_commits_locally() {
        let dir = temp_dir("git-local");
        run(&dir, &["init", "-q"]).unwrap();
        configure(&dir);
        fs::write(dir.join("n.md"), "x").unwrap();
        assert!(sync(&dir).unwrap().contains("ommitted 1 file"));
        assert_eq!(sync(&dir).unwrap(), "No remote configured");
        assert_eq!(repo_name("git@github.com:me/my-notes.git"), "my-notes");
        assert_eq!(repo_name("https://example.com/x/vault/"), "vault");
    }
}
