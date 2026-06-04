use super::diff::detect_changed;
use super::scan::{build_globset, paths_overlap, walk_tree};
use super::types::{CopyOp, DeleteOp, Entry, ExecPlan, PlanItem, SyncPlan};
use crate::config::{Settings, SyncPair};
use std::path::PathBuf;

pub(super) fn build_plan(pair: &SyncPair, settings: &Settings) -> Result<ExecPlan, String> {
    let source = PathBuf::from(&pair.source);
    let dest = PathBuf::from(&pair.destination);
    let min_size = pair.min_file_size;
    let max_size = pair.max_file_size;

    if pair.source.trim().is_empty() || pair.destination.trim().is_empty() {
        return Err("Source ou destination vide".into());
    }
    if paths_overlap(&pair.source, &pair.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    if !source.exists() {
        return Err(format!(
            "Source introuvable : {} (synchro annulée pour éviter d'effacer la destination)",
            pair.source
        ));
    }

    let mut patterns = settings.ignore_patterns.clone();
    patterns.extend(pair.ignore_patterns.clone());
    let ignore = build_globset(&patterns);

    let mut src_map = walk_tree(&source, &ignore);
    let mut dest_map = walk_tree(&dest, &ignore);

    if min_size > 0 || max_size > 0 {
        let filter_size = |e: &Entry| {
            if e.is_dir { return true; }
            if min_size > 0 && e.size < min_size { return false; }
            if max_size > 0 && e.size > max_size { return false; }
            true
        };
        src_map.retain(|_, e| filter_size(e));
        dest_map.retain(|_, e| filter_size(e));
    }

    let dest_total = dest_map.len();

    let mut create_dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut copies: Vec<CopyOp> = Vec::new();
    let mut deletes: Vec<DeleteOp> = Vec::new();
    let mut total_bytes: u64 = 0;

    let dst_abs_of = |rel: &str| -> PathBuf { dest.join(rel) };

    for (k, se) in &src_map {
        match dest_map.get(k) {
            None => {
                if se.is_dir {
                    create_dirs.push((se.rel.clone(), dst_abs_of(&se.rel)));
                } else {
                    total_bytes += se.size;
                    copies.push(CopyOp {
                        rel: se.rel.clone(),
                        src_abs: se.abs.clone(),
                        dst_abs: dst_abs_of(&se.rel),
                        size: se.size,
                        reason: "new".into(),
                        is_new: true,
                    });
                }
            }
            Some(de) => {
                if se.is_dir && de.is_dir {
                } else if !se.is_dir && !de.is_dir {
                    let (changed, reason) = detect_changed(se, de, settings);
                    if changed {
                        total_bytes += se.size;
                        copies.push(CopyOp {
                            rel: se.rel.clone(),
                            src_abs: se.abs.clone(),
                            dst_abs: dst_abs_of(&se.rel),
                            size: se.size,
                            reason,
                            is_new: false,
                        });
                    }
                } else {
                    deletes.push(DeleteOp {
                        rel: se.rel.clone(),
                        abs: dst_abs_of(&se.rel),
                        is_dir: de.is_dir,
                    });
                    if se.is_dir {
                        create_dirs.push((se.rel.clone(), dst_abs_of(&se.rel)));
                    } else {
                        total_bytes += se.size;
                        copies.push(CopyOp {
                            rel: se.rel.clone(),
                            src_abs: se.abs.clone(),
                            dst_abs: dst_abs_of(&se.rel),
                            size: se.size,
                            reason: "new".into(),
                            is_new: true,
                        });
                    }
                }
            }
        }
    }

    let mut extras: Vec<&Entry> = dest_map
        .iter()
        .filter(|(k, _)| !src_map.contains_key(*k))
        .map(|(_, e)| e)
        .collect();
    let extra_entries = extras.len();

    extras.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut last_root: Option<String> = None;
    for e in extras {
        let covered = match &last_root {
            Some(r) => e.rel.starts_with(&format!("{r}/")),
            None => false,
        };
        if covered {
            continue;
        }
        deletes.push(DeleteOp {
            rel: e.rel.clone(),
            abs: dst_abs_of(&e.rel),
            is_dir: e.is_dir,
        });
        last_root = Some(e.rel.clone());
    }

    create_dirs.sort_by_key(|(rel, _)| rel.matches('/').count());

    let delete_pct = if dest_total > 0 {
        (extra_entries as u64 * 100 / dest_total as u64) as u32
    } else {
        0
    };
    let threshold = settings.delete_safety_threshold_pct;
    let aborted_safety = extra_entries > 0 && threshold < 100 && delete_pct > threshold;

    Ok(ExecPlan {
        create_dirs,
        copies,
        deletes,
        total_bytes,
        extra_entries,
        dest_total,
        delete_pct,
        aborted_safety,
    })
}

fn plan_to_dto(plan: &ExecPlan) -> SyncPlan {
    let mut to_copy = Vec::new();
    let mut to_overwrite = Vec::new();
    for c in &plan.copies {
        let item = PlanItem {
            rel: c.rel.clone(),
            is_dir: false,
            size: c.size,
            reason: c.reason.clone(),
        };
        if c.is_new {
            to_copy.push(item);
        } else {
            to_overwrite.push(item);
        }
    }
    let to_delete = plan
        .deletes
        .iter()
        .map(|d| PlanItem {
            rel: d.rel.clone(),
            is_dir: d.is_dir,
            size: 0,
            reason: "extra".into(),
        })
        .collect();
    SyncPlan {
        to_create_dir: plan.create_dirs.iter().map(|(r, _)| r.clone()).collect(),
        to_copy,
        to_overwrite,
        to_delete,
        total_bytes: plan.total_bytes,
        delete_pct: plan.delete_pct,
        aborted_safety: plan.aborted_safety,
    }
}

pub fn dry_run(pair: &SyncPair, settings: &Settings) -> Result<SyncPlan, String> {
    let plan = build_plan(pair, settings)?;
    Ok(plan_to_dto(&plan))
}
