use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone)]
pub(super) struct Entry {
    pub rel: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64,
    pub abs: PathBuf,
}

pub(super) struct CopyOp {
    pub rel: String,
    pub src_abs: PathBuf,
    pub dst_abs: PathBuf,
    pub size: u64,
    pub reason: String,
    pub is_new: bool,
}

pub(super) struct DeleteOp {
    pub rel: String,
    pub abs: PathBuf,
    pub is_dir: bool,
}

pub(super) struct ExecPlan {
    pub create_dirs: Vec<(String, PathBuf)>,
    pub copies: Vec<CopyOp>,
    pub deletes: Vec<DeleteOp>,
    pub total_bytes: u64,
    pub extra_entries: usize,
    pub dest_total: usize,
    pub delete_pct: u32,
    pub aborted_safety: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub rel: String,
    pub is_dir: bool,
    pub size: u64,
    pub reason: String,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub to_create_dir: Vec<String>,
    pub to_copy: Vec<PlanItem>,
    pub to_overwrite: Vec<PlanItem>,
    pub to_delete: Vec<PlanItem>,
    pub total_bytes: u64,
    pub delete_pct: u32,
    pub aborted_safety: bool,
}

#[derive(Default, Clone, Copy)]
pub(super) struct SyncOutcome {
    pub copied: u64,
    pub updated: u64,
    pub deleted: u64,
    pub errors: u64,
    pub aborted_safety: bool,
}
