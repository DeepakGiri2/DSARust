//! Hot reload. Editing `trace.rhai` and seeing the animation change without
//! restarting — let alone recompiling — is the whole point of keeping content
//! outside the binary, so the watcher is part of the contract, not a nicety.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// What changed on disk, already mapped from paths to meaning.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// One problem pack (`problems/<slug>/…`).
    Pack(String),
    /// `catalog.toml` — the problem list itself.
    Catalog,
    /// `languages.toml`.
    Languages,
    /// A shared `lib/*.rhai` helper: every script must be recompiled.
    Prelude,
}

pub struct ContentWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Change>,
    root: PathBuf,
}

impl ContentWatcher {
    pub fn new(root: impl AsRef<Path>) -> notify::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let (tx, rx) = channel();
        let watch_root = root.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let Ok(event) = res else { return };
                if !matches!(
                    event.kind,
                    notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_)
                ) {
                    return;
                }
                for path in event.paths {
                    if let Some(change) = classify(&watch_root, &path) {
                        let _ = tx.send(change);
                    }
                }
            })?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Drain pending changes, deduplicated. Editors save in bursts (write to a
    /// temp file, rename, touch the directory), so a single keystroke-save can
    /// produce a dozen events for one file.
    pub fn poll(&self) -> BTreeSet<Change> {
        let mut out = BTreeSet::new();
        // Both "nothing pending" and "sender gone" mean the same thing here:
        // stop draining and report what we have.
        while let Ok(change) = self.rx.try_recv() {
            out.insert(change);
        }
        out
    }

    /// Blocking variant for headless tools such as `xtask watch`.
    pub fn poll_timeout(&self, timeout: Duration) -> BTreeSet<Change> {
        let mut out = BTreeSet::new();
        if let Ok(c) = self.rx.recv_timeout(timeout) {
            out.insert(c);
            // Let the rest of the burst land before reporting.
            std::thread::sleep(Duration::from_millis(60));
            out.extend(self.poll());
        }
        out
    }
}

/// Map a changed path to the reload it implies. Editor scratch files
/// (`.swp`, `4913`, `~` backups) are ignored so a save does not trigger two
/// reloads.
fn classify(root: &Path, path: &Path) -> Option<Change> {
    let name = path.file_name()?.to_str()?;
    if name.starts_with('.') || name.ends_with('~') || name.ends_with(".tmp") {
        return None;
    }
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string());
    match parts.next()?.as_str() {
        "catalog.toml" => Some(Change::Catalog),
        "languages.toml" => Some(Change::Languages),
        "lib" => Some(Change::Prelude),
        "problems" => parts.next().map(Change::Pack),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_map_to_the_right_reload() {
        let root = Path::new("/c");
        assert_eq!(
            classify(root, &root.join("catalog.toml")),
            Some(Change::Catalog)
        );
        assert_eq!(
            classify(root, &root.join("languages.toml")),
            Some(Change::Languages)
        );
        assert_eq!(
            classify(root, &root.join("lib/common.rhai")),
            Some(Change::Prelude)
        );
        assert_eq!(
            classify(root, &root.join("problems/two-sum/trace.rhai")),
            Some(Change::Pack("two-sum".into()))
        );
        assert_eq!(
            classify(root, &root.join("problems/two-sum/code/go.txt")),
            Some(Change::Pack("two-sum".into()))
        );
    }

    #[test]
    fn editor_scratch_files_are_ignored() {
        let root = Path::new("/c");
        assert_eq!(
            classify(root, &root.join("problems/two-sum/.trace.rhai.swp")),
            None
        );
        assert_eq!(
            classify(root, &root.join("problems/two-sum/trace.rhai~")),
            None
        );
        assert_eq!(
            classify(root, &root.join("problems/two-sum/trace.rhai.tmp")),
            None
        );
    }

    #[test]
    fn unrelated_paths_are_ignored() {
        let root = Path::new("/c");
        assert_eq!(classify(root, &root.join("README.md")), None);
        assert_eq!(classify(root, Path::new("/elsewhere/x.rhai")), None);
    }

    #[test]
    fn a_real_edit_is_observed() {
        let dir = std::env::temp_dir().join(format!("dsa-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("problems/demo")).unwrap();
        std::fs::write(dir.join("catalog.toml"), "").unwrap();
        let w = ContentWatcher::new(&dir).unwrap();
        std::thread::sleep(Duration::from_millis(120));
        std::fs::write(dir.join("problems/demo/trace.rhai"), "fn trace(i) {}").unwrap();

        let mut changes = BTreeSet::new();
        for _ in 0..40 {
            changes.extend(w.poll());
            if !changes.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            changes.contains(&Change::Pack("demo".into())),
            "got {changes:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
