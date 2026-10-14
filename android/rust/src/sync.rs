//! Git sync for Android, built on libgit2 because there is no `git` binary on the device.
//!
//! A sync commits every change, fetches, merges the remote branch and pushes. Unlike the
//! desktop (which rebases and stops on conflicts), conflicts are resolved automatically by
//! keeping both versions: ours stays at the original path and theirs is saved next to it as
//! "Note (conflict).md", so nobody has to resolve git conflict markers on a phone.

use git2::build::CheckoutBuilder;
use git2::{
    Commit, Cred, ErrorCode, FetchOptions, IndexAddOption, IndexEntry, ObjectType, Oid, PushOptions, Remote,
    RemoteCallbacks, Repository, RepositoryInitOptions, Signature, TreeWalkMode, TreeWalkResult,
};
use scrap_core::vault;
use std::cell::RefCell;
use std::fs;
use std::path::Path;

pub struct Config {
    pub url: String,
    pub username: String,
    pub token: String,
    pub branch: String,
    pub name: String,
    pub email: String,
    /// Shown in commit messages, e.g. "Pixel 8".
    pub device: String,
}

type Res<T> = Result<T, String>;

fn g(e: git2::Error) -> String {
    e.message().to_string()
}

fn io(e: std::io::Error) -> String {
    e.to_string()
}

pub fn sync(root: &Path, cfg: &Config) -> Res<String> {
    let repo = open_or_init(root, &cfg.branch)?;
    let branch = current_branch(&repo).unwrap_or_else(|| cfg.branch.clone());
    let local_ref = format!("refs/heads/{branch}");
    let tracking_ref = format!("refs/remotes/origin/{branch}");
    let mut done = Vec::new();

    let mut remote = ensure_remote(&repo, &cfg.url)?;
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks(cfg));
    remote
        .fetch(&[] as &[&str], Some(&mut fetch_opts), None)
        .map_err(|e| format!("Couldn't reach the repository: {}", e.message()))?;
    let remote_oid = repo.refname_to_id(&tracking_ref).ok();

    // First sync on this device: start from what the remote already has.
    if head_commit(&repo)?.is_none() {
        if let Some(oid) = remote_oid {
            adopt(&repo, root, oid, &local_ref)?;
            done.push("downloaded notes".to_string());
        }
    }

    if let Some(n) = commit_all(&repo, cfg)? {
        done.push(format!("committed {n} file{}", plural(n)));
    }

    if let (Some(local), Some(theirs)) = (head_commit(&repo)?, remote_oid) {
        if local.id() != theirs {
            let (ahead, behind) = repo
                .graph_ahead_behind(local.id(), theirs)
                .map_err(|_| "This vault and the repository have unrelated histories.".to_string())?;
            if behind > 0 {
                if ahead == 0 {
                    move_branch(&repo, &local_ref, theirs)?;
                } else {
                    let theirs = repo.find_commit(theirs).map_err(g)?;
                    let kept = merge(&repo, root, &local, &theirs, cfg)?;
                    if kept > 0 {
                        done.push(format!("kept both versions of {kept} conflicting note{}", plural(kept)));
                    }
                }
                done.push(format!("pulled {behind} change{}", plural(behind)));
            }
        }
    }

    if let Some(local) = head_commit(&repo)? {
        let ahead = match remote_oid {
            Some(theirs) if theirs == local.id() => 0,
            Some(theirs) => repo.graph_ahead_behind(local.id(), theirs).map_or(1, |(a, _)| a),
            None => 1,
        };
        if ahead > 0 {
            push(&mut remote, cfg, &local_ref)?;
            repo.reference(&tracking_ref, local.id(), true, "scrap: pushed").map_err(g)?;
            done.push("pushed".to_string());
        }
    }

    Ok(summary(done))
}

fn open_or_init(root: &Path, branch: &str) -> Res<Repository> {
    if root.join(".git").exists() {
        return Repository::open(root).map_err(g);
    }
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head(branch);
    let repo = Repository::init_opts(root, &opts).map_err(g)?;
    // Keep notes byte-identical across operating systems.
    repo.config().and_then(|mut c| c.set_bool("core.autocrlf", false)).map_err(g)?;
    let ignore = root.join(".gitignore");
    if !ignore.exists() {
        fs::write(&ignore, format!("{}/\n", vault::TRASH)).map_err(io)?;
    }
    Ok(repo)
}

fn current_branch(repo: &Repository) -> Option<String> {
    let head = repo.find_reference("HEAD").ok()?;
    head.symbolic_target()?.strip_prefix("refs/heads/").map(str::to_string)
}

fn head_commit(repo: &Repository) -> Res<Option<Commit<'_>>> {
    match repo.head() {
        Ok(head) => head.peel_to_commit().map(Some).map_err(g),
        Err(e) if matches!(e.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => Ok(None),
        Err(e) => Err(g(e)),
    }
}

fn ensure_remote<'r>(repo: &'r Repository, url: &str) -> Res<Remote<'r>> {
    match repo.find_remote("origin") {
        Ok(r) if r.url() == Some(url) => Ok(r),
        Ok(_) => {
            repo.remote_set_url("origin", url).map_err(g)?;
            repo.find_remote("origin").map_err(g)
        }
        Err(_) => repo.remote("origin", url).map_err(g),
    }
}

fn callbacks(cfg: &Config) -> RemoteCallbacks<'_> {
    let mut cb = RemoteCallbacks::new();
    let mut attempts = 0;
    cb.credentials(move |_url, _user, _allowed| {
        attempts += 1;
        // libgit2 keeps asking while credentials are rejected; give up after one try.
        if attempts > 1 || cfg.token.is_empty() {
            return Err(git2::Error::from_str("authentication failed: check the username and access token"));
        }
        let user = if cfg.username.is_empty() { "git" } else { &cfg.username };
        Cred::userpass_plaintext(user, &cfg.token)
    });
    cb
}

/// Points this (empty) vault's branch at the remote commit and checks it out. Local files
/// that would be overwritten are kept as "Name (local).md".
fn adopt(repo: &Repository, root: &Path, oid: Oid, local_ref: &str) -> Res<()> {
    let tree = repo.find_commit(oid).and_then(|c| c.tree()).map_err(g)?;
    let mut files = Vec::new();
    tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(ObjectType::Blob) {
            if let Some(name) = entry.name() {
                files.push((format!("{dir}{name}"), entry.id()));
            }
        }
        TreeWalkResult::Ok
    })
    .map_err(g)?;

    // An untouched welcome note isn't worth uploading.
    let welcome = root.join("Welcome.md");
    if fs::read_to_string(&welcome).is_ok_and(|t| t == vault::WELCOME) {
        fs::remove_file(&welcome).map_err(io)?;
    }
    for (rel, id) in files {
        let path = root.join(&rel);
        if !path.is_file() {
            continue;
        }
        let blob = repo.find_blob(id).map_err(g)?;
        if fs::read(&path).map_err(io)? != blob.content() {
            let aside = free_name(&rel, "local", |p| root.join(p).exists());
            fs::rename(&path, root.join(aside)).map_err(io)?;
        }
    }
    move_branch(repo, local_ref, oid)?;
    repo.set_head(local_ref).map_err(g)
}

/// Moves the branch to `oid` and updates the working tree to match. Only called when the
/// working tree has nothing uncommitted, so a forced checkout can't lose edits.
fn move_branch(repo: &Repository, local_ref: &str, oid: Oid) -> Res<()> {
    repo.reference(local_ref, oid, true, "scrap: update").map_err(g)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force())).map_err(g)
}

/// Stages everything (respecting .gitignore) and commits. Returns the number of changed files.
fn commit_all(repo: &Repository, cfg: &Config) -> Res<Option<usize>> {
    let mut index = repo.index().map_err(g)?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None).map_err(g)?;
    index.update_all(["*"], None).map_err(g)?;
    index.write().map_err(g)?;
    let tree = repo.find_tree(index.write_tree().map_err(g)?).map_err(g)?;

    let parent = head_commit(repo)?;
    let old_tree = match &parent {
        Some(p) => Some(p.tree().map_err(g)?),
        None => None,
    };
    let diff = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&tree), None).map_err(g)?;
    let files: Vec<String> = diff
        .deltas()
        .filter_map(|d| d.new_file().path().or(d.old_file().path()).map(|p| p.to_string_lossy().into_owned()))
        .collect();
    if files.is_empty() {
        return Ok(None);
    }
    let sig = signature(cfg)?;
    let parents: Vec<&Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, &commit_message(&files, &cfg.device), &tree, &parents)
        .map_err(g)?;
    Ok(Some(files.len()))
}

/// Merges `theirs` into HEAD, keeping both versions of conflicting files.
/// Returns how many files had conflicts.
fn merge(repo: &Repository, root: &Path, ours: &Commit, theirs: &Commit, cfg: &Config) -> Res<usize> {
    let mut index = repo.merge_commits(ours, theirs, None).map_err(g)?;
    let mut kept = 0;
    if index.has_conflicts() {
        let conflicts = index.conflicts().map_err(g)?.collect::<Result<Vec<_>, _>>().map_err(g)?;
        for c in conflicts {
            let Some(any) = c.our.as_ref().or(c.their.as_ref()).or(c.ancestor.as_ref()) else { continue };
            let path = String::from_utf8_lossy(&any.path).into_owned();
            index.conflict_remove(Path::new(&path)).map_err(g)?;
            match (c.our, c.their) {
                (Some(ours), Some(theirs)) => {
                    let copy = free_name(&path, "conflict", |p| {
                        root.join(p).exists() || index.get_path(Path::new(p), 0).is_some()
                    });
                    index.add(&stage0(ours, None)).map_err(g)?;
                    index.add(&stage0(theirs, Some(&copy))).map_err(g)?;
                    kept += 1;
                }
                // Edited on one side, deleted on the other: keep the edit.
                (Some(entry), None) | (None, Some(entry)) => index.add(&stage0(entry, None)).map_err(g)?,
                (None, None) => {}
            }
        }
    }
    let tree = repo.find_tree(index.write_tree_to(repo).map_err(g)?).map_err(g)?;
    let sig = signature(cfg)?;
    let message = if kept == 0 {
        "Merge remote changes".to_string()
    } else {
        format!("Merge remote changes (kept both versions of {kept} note{})", plural(kept))
    };
    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[ours, theirs]).map_err(g)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force())).map_err(g)?;
    Ok(kept)
}

/// A resolved (stage 0) copy of an index entry, optionally under another path.
fn stage0(mut entry: IndexEntry, path: Option<&str>) -> IndexEntry {
    if let Some(p) = path {
        entry.path = p.as_bytes().to_vec();
    }
    const STAGE_AND_NAME_LEN: u16 = 0x3FFF;
    entry.flags = (entry.flags & !STAGE_AND_NAME_LEN) | entry.path.len().min(0xFFF) as u16;
    entry
}

fn push(remote: &mut Remote, cfg: &Config, local_ref: &str) -> Res<()> {
    let rejected = RefCell::new(None);
    {
        let mut cb = callbacks(cfg);
        cb.push_update_reference(|name, status| {
            if let Some(s) = status {
                *rejected.borrow_mut() = Some(format!("{name}: {s}"));
            }
            Ok(())
        });
        let mut opts = PushOptions::new();
        opts.remote_callbacks(cb);
        remote
            .push(&[format!("{local_ref}:{local_ref}")], Some(&mut opts))
            .map_err(|e| format!("Push failed: {}", e.message()))?;
    }
    match rejected.into_inner() {
        Some(reason) => Err(format!("The repository rejected the push ({reason}). Sync again to merge.")),
        None => Ok(()),
    }
}

fn signature(cfg: &Config) -> Res<Signature<'static>> {
    let name = if cfg.name.trim().is_empty() { "Scrap" } else { cfg.name.trim() };
    let email = if cfg.email.trim().is_empty() { "scrap@localhost" } else { cfg.email.trim() };
    Signature::now(name, email).map_err(g)
}

/// "notes/Idea.md" -> "notes/Idea (tag).md", numbered if that is taken.
fn free_name(rel: &str, tag: &str, taken: impl Fn(&str) -> bool) -> String {
    let (stem, ext) = match rel.rsplit_once('.') {
        Some((s, e)) if !e.contains('/') => (s, format!(".{e}")),
        _ => (rel, String::new()),
    };
    let mut n = 1;
    loop {
        let candidate =
            if n == 1 { format!("{stem} ({tag}){ext}") } else { format!("{stem} ({tag} {n}){ext}") };
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn commit_message(files: &[String], device: &str) -> String {
    let names: Vec<&str> = files.iter().take(3).map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
    let mut msg = format!("Update {}", names.join(", "));
    if files.len() > 3 {
        msg += &format!(" and {} more", files.len() - 3);
    }
    if !device.is_empty() {
        msg += &format!(" (from {device})");
    }
    msg
}

fn plural<N: PartialEq + From<u8>>(n: N) -> &'static str {
    if n == N::from(1) { "" } else { "s" }
}

fn summary(done: Vec<String>) -> String {
    if done.is_empty() {
        return "Already up to date".to_string();
    }
    let s = done.join(", ");
    let mut chars = s.chars();
    chars.next().map(|f| f.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrap-sync-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn config(remote: &Path, device: &str) -> Config {
        Config {
            url: remote.to_string_lossy().into_owned(),
            username: String::new(),
            token: String::new(),
            branch: "main".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            device: device.into(),
        }
    }

    fn read(p: PathBuf) -> String {
        fs::read_to_string(p).unwrap()
    }

    #[test]
    fn phones_sync_through_a_remote() {
        let tmp = temp_dir("phones");
        let bare = tmp.join("remote.git");
        Repository::init_bare(&bare).unwrap();
        let (a, b) = (tmp.join("a"), tmp.join("b"));

        // Phone A: fresh vault with the welcome note and one note; first sync pushes.
        vault::seed(&a).unwrap();
        fs::write(a.join("Ideas.md"), "from a\n").unwrap();
        let msg = sync(&a, &config(&bare, "Phone A")).unwrap();
        assert!(msg.contains("ommitted") && msg.contains("pushed"), "{msg}");
        assert_eq!(sync(&a, &config(&bare, "Phone A")).unwrap(), "Already up to date");

        // Phone B: its own fresh vault plus a clashing Ideas.md; it adopts the remote.
        vault::seed(&b).unwrap();
        fs::write(b.join("Ideas.md"), "from b\n").unwrap();
        let msg = sync(&b, &config(&bare, "Phone B")).unwrap();
        assert!(msg.starts_with("Downloaded notes"), "{msg}");
        assert_eq!(read(b.join("Ideas.md")), "from a\n");
        assert_eq!(read(b.join("Ideas (local).md")), "from b\n");
        assert!(b.join(".gitignore").exists());

        // A picks up B's local copy.
        sync(&a, &config(&bare, "Phone A")).unwrap();
        assert_eq!(read(a.join("Ideas (local).md")), "from b\n");

        // Both edit the same note: both versions survive on both phones.
        fs::write(a.join("Ideas.md"), "edited on a\n").unwrap();
        sync(&a, &config(&bare, "Phone A")).unwrap();
        fs::write(b.join("Ideas.md"), "edited on b\n").unwrap();
        fs::create_dir_all(b.join("sub")).unwrap();
        fs::write(b.join("sub/New.md"), "new\n").unwrap();
        let msg = sync(&b, &config(&bare, "Phone B")).unwrap();
        assert!(msg.contains("kept both versions of 1"), "{msg}");
        assert_eq!(read(b.join("Ideas.md")), "edited on b\n");
        assert_eq!(read(b.join("Ideas (conflict).md")), "edited on a\n");
        sync(&a, &config(&bare, "Phone A")).unwrap();
        assert_eq!(read(a.join("Ideas.md")), "edited on b\n");
        assert_eq!(read(a.join("Ideas (conflict).md")), "edited on a\n");
        assert_eq!(read(a.join("sub/New.md")), "new\n");

        // Deletions and the trash folder.
        fs::create_dir_all(a.join(vault::TRASH)).unwrap();
        fs::rename(a.join("Ideas (conflict).md"), a.join(vault::TRASH).join("Ideas (conflict).md")).unwrap();
        sync(&a, &config(&bare, "Phone A")).unwrap();
        sync(&b, &config(&bare, "Phone B")).unwrap();
        assert!(!b.join("Ideas (conflict).md").exists());
        assert!(!b.join(vault::TRASH).exists(), "trash is not synced");
        assert_eq!(sync(&b, &config(&bare, "Phone B")).unwrap(), "Already up to date");
    }

    #[test]
    fn wrong_remote_is_reported() {
        let dir = temp_dir("bad-remote");
        fs::write(dir.join("n.md"), "x").unwrap();
        let err = sync(&dir, &config(&dir.join("missing.git"), "")).unwrap_err();
        assert!(err.starts_with("Couldn't reach the repository"), "{err}");
    }

    #[test]
    fn names_for_kept_copies() {
        assert_eq!(free_name("a/Note.md", "conflict", |_| false), "a/Note (conflict).md");
        assert_eq!(free_name("Note.md", "local", |p| p == "Note (local).md"), "Note (local 2).md");
        assert_eq!(free_name("v1.0/README", "x", |_| false), "v1.0/README (x)");
    }
}
