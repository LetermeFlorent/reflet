use super::types::Entry;
use crate::config::Settings;
use std::path::Path;

fn blake3_of(path: &Path) -> Option<[u8; 32]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher).ok()?;
    Some(*hasher.finalize().as_bytes())
}

pub(super) fn detect_changed(src: &Entry, dst: &Entry, settings: &Settings) -> (bool, String) {
    if src.size != dst.size {
        return (true, "size".into());
    }
    if (src.mtime - dst.mtime).abs() > settings.mtime_tolerance_sec {
        return (true, "mtime".into());
    }
    if settings.verify_by_content == "blake3" {
        match (blake3_of(&src.abs), blake3_of(&dst.abs)) {
            (Some(a), Some(b)) if a != b => return (true, "content".into()),
            _ => {}
        }
    }
    (false, String::new())
}
