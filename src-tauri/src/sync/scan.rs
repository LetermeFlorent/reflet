use super::types::Entry;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

fn norm_key(rel: &str) -> String {
    if cfg!(windows) {
        rel.to_lowercase()
    } else {
        rel.to_string()
    }
}

fn norm_path(p: &str) -> String {
    let pb = PathBuf::from(p);
    let c = dunce::canonicalize(&pb).unwrap_or(pb);
    let s = c.to_string_lossy().to_string();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

pub fn paths_overlap(a: &str, b: &str) -> bool {
    let (na, nb) = (norm_path(a), norm_path(b));
    if na == nb {
        return true;
    }
    nested_in(&na, &nb) || nested_in(&nb, &na)
}

fn nested_in(child: &str, parent: &str) -> bool {
    let sep = std::path::MAIN_SEPARATOR as u8;
    child.len() > parent.len()
        && child.starts_with(parent)
        && child.as_bytes()[parent.len()] == sep
}

pub(super) fn build_globset(patterns: &[String]) -> GlobSet {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        if let Ok(g) = Glob::new(p) {
            b.add(g);
        }
    }
    b.build().unwrap_or_else(|_| GlobSet::empty())
}

fn is_reparse_point(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if let Ok(md) = entry.metadata() {
            return md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        }
    }
    false
}

pub(super) fn walk_tree(root: &Path, ignore: &GlobSet) -> BTreeMap<String, Entry> {
    let mut map = BTreeMap::new();
    if !root.exists() {
        return map;
    }
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path();
            if p == root {
                return true;
            }
            if is_reparse_point(e) {
                return false;
            }
            match p.strip_prefix(root) {
                Ok(rel) => {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    !ignore.is_match(&rel_str)
                }
                Err(_) => false,
            }
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == root {
            continue;
        }
        let ft = entry.file_type();
        if is_reparse_point(&entry) {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let is_dir = ft.is_dir();
        map.insert(
            norm_key(&rel),
            Entry {
                rel,
                is_dir,
                size: if is_dir { 0 } else { meta.len() },
                mtime,
                abs: path.to_path_buf(),
            },
        );
    }
    map
}
