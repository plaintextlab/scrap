//! JNI bridge between the Kotlin UI (`dev.plaintextlab.scrap.Core`) and scrap-core.
//!
//! Every function takes and returns Java strings; structured results are JSON.
//! Errors are thrown as `java.io.IOException`.

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use scrap_core::links::{wikilinks_to_md, LinkStyle};
use scrap_core::vault::{self, Vault};
use serde_json::json;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::Mutex;

static VAULT: Mutex<Option<Vault>> = Mutex::new(None);

type Res = Result<String, String>;

fn with_vault<T>(f: impl FnOnce(&mut Vault) -> Result<T, String>) -> Result<T, String> {
    let mut guard = VAULT.lock().map_err(|_| "vault lock poisoned".to_string())?;
    let v = guard.as_mut().ok_or("no vault open")?;
    f(v)
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
        let notes: Vec<_> = v.notes.iter().map(|n| json!({ "rel": n.rel, "name": n.name })).collect();
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
    with_vault(|v| v.save(&rel, &text).map(|_| String::new()).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_createNote(folder, name) {
    with_vault(|v| v.create_note(&folder, &name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_createFolder(parent, name) {
    with_vault(|v| v.create_folder(&parent, &name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_rename(rel, name) {
    with_vault(|v| v.rename(&rel, &name).map_err(io))
});

export!(Java_dev_plaintextlab_scrap_Core_delete(rel) {
    with_vault(|v| v.delete(&rel).map(|_| String::new()).map_err(io))
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
    Ok(wikilinks_to_md(&text, LinkStyle::Encoded))
});
