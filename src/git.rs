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
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.quotepath=false"]);
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    cmd.args(args)
        // Fail instead of hanging on a terminal prompt nobody can see.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().map_err(|e| format!("Could not run git: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
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

/// Commit everything, rebase onto the remote, push.
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
                if let Err(e) = run(root, &["rebase", "-q", "--autostash", &upstream]) {
                    let _ = run(root, &["rebase", "--abort"]);
                    return Err(format!(
                        "Sync stopped: local and remote changes conflict. Nothing was lost; \
                         your changes are committed locally. Resolve with git in a terminal.\n\n{e}"
                    ));
                }
                done.push(format!("pulled {behind} commit{}", if behind == 1 { "" } else { "s" }));
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
    use crate::vault::tests::temp_dir;
    use std::fs;

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

        // Same line edited on both sides: sync stops cleanly, nothing lost.
        fs::write(a.join("one.md"), "edited on a\n").unwrap();
        sync(&a).unwrap();
        fs::write(b.join("one.md"), "edited on b\n").unwrap();
        let err = sync(&b).unwrap_err();
        assert!(err.contains("conflict"), "{err}");
        assert!(!b.join(".git/rebase-merge").exists() && !b.join(".git/rebase-apply").exists());
        assert_eq!(fs::read_to_string(b.join("one.md")).unwrap(), "edited on b\n");
        assert_eq!(status(&b).ahead, 1);
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
