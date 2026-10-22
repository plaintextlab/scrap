//! JNI bridge between the Kotlin UI (`dev.plaintextlab.scrap.Core`) and scrap-core.
//!
//! Every function takes and returns Java strings; structured results are JSON.
//! Errors are thrown as `java.io.IOException`.

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use scrap_core::attachments;
use scrap_core::colors;
use scrap_core::meta;
use scrap_core::links::{wikilinks_to_md, LinkStyle};
use scrap_core::vault::{self, Vault};
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Mutex;

mod sync;

static VAULT: Mutex<Option<Vault>> = Mutex::new(None);
/// Held while files in the vault are written, including for a whole sync, so edits can't
/// land in the middle of a merge. Reads and search only need `VAULT`, so they stay fast
/// while a sync is talking to the network. Always taken before `VAULT`.
static DISK: Mutex<()> = Mutex::new(());

type Res = Result<String, String>;

fn with_vault<T>(f: impl FnOnce(&mut Vault) -> Result<T, String>) -> Result<T, String> {
    let mut guard = VAULT.lock().map_err(|_| "vault lock poisoned".to_string())?;
    let v = guard.as_mut().ok_or("no vault open")?;
    f(v)
}

/// `with_vault` for operations that change files on disk.
fn writing_vault<T>(f: impl FnOnce(&mut Vault) -> Result<T, String>) -> Result<T, String> {
    let _disk = DISK.lock().map_err(|_| "disk lock poisoned".to_string())?;
    with_vault(f)
}

fn io(e: std::io::Error) -> String {
    e.to_string()
}

fn string_arg(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(Into::into).map_err(|e| e.to_string())
}

fn finish(env: &mut JNIEnv, result: std::thread::Result<Res>) -> jstring {
    let result = result.unwrap_or_else(|_| Err("internal error".into()));
    match result {
        Ok(s) => env.new_string(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
        Err(e) => {
            let _ = env.throw_new("java/io/IOException", e);
            std::ptr::null_mut()
        }
    }
}

/// Declares `Java_dev_plaintextlab_scrap_Core_<name>` taking string arguments.
macro_rules! export {
    ($name:ident ( $($arg:ident),* ) $body:block) => {
        #[no_mangle]
        pub extern "system" fn $name<'l>(mut env: JNIEnv<'l> , _class: JClass<'l> $(, $arg: JString<'l>)*) -> jstring {
            let result = catch_unwind(AssertUnwindSafe(|| -> Res {
                $(let $arg = string_arg(&mut env, &$arg)?;)*
                $body
            }));
            finish(&mut env, result)
        }
    };
}

export!(Java_dev_plaintextlab_scrap_Core_open(root) {
    let root = PathBuf::from(root);
    vault::seed(&root).map_err(io)?;
    let v = Vault::open(root).map_err(io)?;
    *VAULT.lock().map_err(|_| "vault lock poisoned".to_string())? = Some(v);
    Ok(String::new())
});

export!(Java_dev_plaintextlab_scrap_Core_rescan() {
    with_vault(|v| {
        v.rescan();
        Ok(String::new())
    })
});

export!(Java_dev_plaintextlab_scrap_Core_list() {
    with_vault(|v| {
        let mut folders = Vec::new();
        collect_folders(&v.tree, &mut folders);
        let notes: Vec<_> = v
            .notes
            .iter()
            .map(|n| json!({ "rel": n.rel, "name": n.name, "modified": n.modified, "pinned": n.pinned(), "color": n.color() }))
            .collect();
        Ok(json!({ "folders": folders, "notes": notes }).to_string())
    })
});

fn collect_folders(dir: &vault::Dir, out: &mut Vec<String>) {
    for d in &dir.dirs {
        out.push(d.rel.clone());
        collect_folders(d, out);
    }
}

export!(Java_dev_plaintextlab_scrap_Core_read(rel) {
    with_vault(|v| v.read(&rel).map(|t| t.replace("\r\n", "\n")).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_save(rel, text) {
    writing_vault(|v| v.save(&rel, &text).map(|_| String::new()).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_createNote(folder, name) {
    writing_vault(|v| v.create_note(&folder, &name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_createFolder(parent, name) {
    writing_vault(|v| v.create_folder(&parent, &name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_rename(rel, name) {
    writing_vault(|v| v.rename(&rel, &name).map_err(io))
});

// Moves a note or folder into another folder ("" = top level); returns its new path.
export!(Java_dev_plaintextlab_scrap_Core_move(rel, folder) {
    writing_vault(|v| v.move_to(&rel, &folder).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_delete(rel) {
    writing_vault(|v| v.delete(&rel).map(|_| String::new()).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_trash() {
    with_vault(|v| {
        let items: Vec<_> = v
            .trash()
            .into_iter()
            .map(|t| json!({ "name": t.name, "original": t.original, "dir": t.is_dir, "deleted": t.deleted }))
            .collect();
        Ok(json!(items).to_string())
    })
});

// Returns where the item was restored to.
export!(Java_dev_plaintextlab_scrap_Core_restore(name) {
    writing_vault(|v| v.restore(&name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_deleteForever(name) {
    writing_vault(|v| v.delete_forever(&name).map(|_| String::new()).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_emptyTrash() {
    writing_vault(|v| v.empty_trash().map(|_| String::new()).map_err(io))
});

// Path of the note a wiki-link target points to, or "" when it doesn't exist.
export!(Java_dev_plaintextlab_scrap_Core_resolve(target) {
    with_vault(|v| Ok(v.resolve(&target).map(|i| v.notes[i].rel.clone()).unwrap_or_default()))
});

export!(Java_dev_plaintextlab_scrap_Core_backlinks(rel) {
    with_vault(|v| {
        let links: Vec<_> = v
            .index_of(&rel)
            .map(|i| v.backlinks(i))
            .unwrap_or_default()
            .into_iter()
            .map(|j| json!({ "rel": v.notes[j].rel, "name": v.notes[j].name }))
            .collect();
        Ok(json!(links).to_string())
    })
});

export!(Java_dev_plaintextlab_scrap_Core_outgoing(text) {
    with_vault(|v| {
        let mut targets = vault::extract_links(&text);
        targets.dedup();
        let links: Vec<_> = targets
            .iter()
            .map(|t| json!({ "target": t, "exists": v.resolve(t).is_some() }))
            .collect();
        Ok(json!(links).to_string())
    })
});

export!(Java_dev_plaintextlab_scrap_Core_search(query) {
    with_vault(|v| {
        let hits: Vec<_> = v
            .search(&query, 200)
            .into_iter()
            .map(|h| json!({ "rel": v.notes[h.note].rel, "name": v.notes[h.note].name, "text": h.text }))
            .collect();
        Ok(json!(hits).to_string())
    })
});

// Markdown with `[[wiki links]]` rewritten as `wiki:` links for the renderer.
export!(Java_dev_plaintextlab_scrap_Core_preview(text) {
    Ok(guard_spans(&wikilinks_to_md(&text, LinkStyle::Encoded)))
});

/// A line starting with a colour `<span>` must stay an ordinary paragraph (with the span
/// as inline HTML the renderer can colour), not be taken for a raw HTML block; an
/// invisible zero-width space in front of the tag makes sure of that.
fn guard_spans(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.split_inclusive('\n') {
        let rest = line.trim_start();
        if colors::open_tag(rest).is_some() {
            out.push_str(&line[..line.len() - rest.len()]);
            out.push('\u{200B}');
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
    }
    out
}

// A copy of a note next to it ("<name> copy"); returns its path.
export!(Java_dev_plaintextlab_scrap_Core_duplicate(rel) {
    writing_vault(|v| v.duplicate(&rel).map_err(io))
});

// A copy of a note in another folder; returns its path.
export!(Java_dev_plaintextlab_scrap_Core_copyTo(rel, folder) {
    writing_vault(|v| v.copy_to(&rel, &folder).map_err(io))
});

// A note's properties: {"pinned", "color", "tags" (all), "frontTags" (editable ones)}.
export!(Java_dev_plaintextlab_scrap_Core_noteMeta(text) {
    Ok(json!({
        "pinned": meta::pinned(&text),
        "color": meta::color(&text),
        "tags": meta::tags(&text),
        "frontTags": meta::front_tags(&text),
    })
    .to_string())
});

// The note's text with `pinned` set ("true") or removed (anything else).
export!(Java_dev_plaintextlab_scrap_Core_setPinned(text, pinned) {
    Ok(meta::set_pinned(&text, pinned == "true"))
});

// The note's text with its background colour set, or removed ("").
export!(Java_dev_plaintextlab_scrap_Core_setColor(text, color) {
    Ok(meta::set_color(&text, Some(color.as_str()).filter(|c| !c.is_empty())))
});

// The note's text with its front matter tags replaced; `tags` is comma-separated.
export!(Java_dev_plaintextlab_scrap_Core_setTags(text, tags) {
    let tags: Vec<String> = tags.split([',', ' ']).map(str::trim).filter(|t| !t.is_empty()).map(String::from).collect();
    Ok(meta::set_tags(&text, &tags))
});

// The note as plain text, for exporting as .txt.
export!(Java_dev_plaintextlab_scrap_Core_plainText(text) {
    Ok(meta::plain_text(&text))
});

// Where an attached file called `name` goes in the vault (a free path in .attachments).
export!(Java_dev_plaintextlab_scrap_Core_attachmentPath(name) {
    with_vault(|v| attachments::free_path_for(&v.root, &name).map_err(io))
});

// The markdown link from the note at `rel` to the attached file at `path`.
export!(Java_dev_plaintextlab_scrap_Core_fileLink(rel, path) {
    Ok(attachments::file_link(&rel, &path))
});

// The note as plain markdown, without colour spans, for sharing.
export!(Java_dev_plaintextlab_scrap_Core_stripColors(text) {
    Ok(colors::strip(&text))
});

// Commits, pulls and pushes the vault. `config` is JSON with url, username, token, branch,
// name, email and device.
export!(Java_dev_plaintextlab_scrap_Core_sync(config) {
    let c: serde_json::Value = serde_json::from_str(&config).map_err(|e| e.to_string())?;
    let field = |k: &str| c[k].as_str().unwrap_or_default().trim().to_string();
    let cfg = sync::Config {
        url: field("url"),
        username: field("username"),
        token: field("token"),
        branch: if field("branch").is_empty() { "main".into() } else { field("branch") },
        name: field("name"),
        email: field("email"),
        device: field("device"),
    };
    if cfg.url.is_empty() {
        return Err("No repository configured".into());
    }
    #[cfg(target_os = "android")]
    tls::init()?;
    // A repository other than the open one (e.g. the one just switched away from) is
    // synced in its own folder without touching the open vault.
    let other = field("root");
    if !other.is_empty() {
        return sync::sync(std::path::Path::new(&other), &cfg);
    }
    // The network part can take seconds; don't hold the vault lock (and block search,
    // saving and browsing) while it runs. Sync only touches files on disk.
    let _disk = DISK.lock().map_err(|_| "disk lock poisoned".to_string())?;
    let root = with_vault(|v| Ok(v.root.clone()))?;
    let result = sync::sync(&root, &cfg);
    with_vault(|v| {
        v.rescan();
        Ok(())
    })?;
    result
});

/// libgit2 on Android: trust the system CA certificates and relax checks that assume a
/// desktop environment.
#[cfg(target_os = "android")]
mod tls {
    use std::fs;
    use std::os::raw::{c_int, c_void};
    use std::ptr;
    use std::sync::OnceLock;

    static INIT: OnceLock<Result<(), String>> = OnceLock::new();

    pub fn init() -> Result<(), String> {
        INIT.get_or_init(|| {
            let err = |e: git2::Error| e.message().to_string();
            // SAFETY: runs once, before any other libgit2 use in this process.
            unsafe {
                // App storage may report a different owner than the app's uid.
                git2::opts::set_verify_owner_validation(false).map_err(err)?;
                for level in [git2::ConfigLevel::System, git2::ConfigLevel::XDG, git2::ConfigLevel::Global] {
                    let _ = git2::opts::set_search_path(level, "");
                }
            }
            // Android keeps one PEM file per CA. The bundled OpenSSL is built without stdio
            // and can't open files itself, so each certificate is parsed from memory.
            let mut added = 0;
            for dir in ["/apex/com.android.conscrypt/cacerts", "/system/etc/security/cacerts"] {
                for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                    if let Ok(pem) = fs::read(entry.path()) {
                        added += unsafe { add_certs(&pem) };
                    }
                }
            }
            if added == 0 {
                return Err("No system CA certificates found".into());
            }
            Ok(())
        })
        .clone()
    }

    /// Adds every certificate in a PEM blob to libgit2's trust store. Returns how many.
    unsafe fn add_certs(pem: &[u8]) -> usize {
        use openssl_sys::*;
        let bio = BIO_new_mem_buf(pem.as_ptr() as *const c_void, pem.len() as c_int);
        if bio.is_null() {
            return 0;
        }
        let mut added = 0;
        loop {
            let cert = PEM_read_bio_X509(bio, ptr::null_mut(), None, ptr::null_mut());
            if cert.is_null() {
                break;
            }
            // The store takes its own reference, so ours is released either way.
            if libgit2_sys::git_libgit2_opts(libgit2_sys::GIT_OPT_ADD_SSL_X509_CERT as c_int, cert) == 0 {
                added += 1;
            }
            X509_free(cert);
        }
        BIO_free_all(bio);
        // Reaching the end of the PEM data leaves a "no start line" error queued.
        ERR_clear_error();
        added
    }
}
