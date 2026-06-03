use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionMethod {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub available: bool,
    pub supports_password: bool,
    pub download_url: String,
    pub default_level: u32,
    pub max_level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionConfig {
    pub method: String,
    pub level: u32,
    pub password: Option<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        CompressionConfig {
            method: "off".into(),
            level: 0,
            password: None,
        }
    }
}

impl CompressionConfig {
    pub fn extension(&self) -> &'static str {
        if self.method == "off" {
            return "";
        }
        method_extension(&self.method)
    }
}

fn method_extension(method: &str) -> &'static str {
    match method {
        "zip" => ".zip",
        "7z" => ".7z",
        "zstd" => ".zst",
        "gzip" => ".gz",
        "xz" => ".xz",
        "lz4" => ".lz4",
        _ => "",
    }
}

fn has_binary(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok()
}

fn has_powershell_zip() -> bool {
    #[cfg(windows)]
    {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "& {Get-Command Compress-Archive -ErrorAction SilentlyContinue}"])
            .output();
        out.map(|o| o.status.success()).unwrap_or(false)
    }
    #[cfg(not(windows))]
    false
}

/// Detect available compression methods on the current system.
/// Returns ALL known methods with their availability flag.
pub fn detect_methods() -> Vec<CompressionMethod> {
    let mut methods: Vec<CompressionMethod> = Vec::new();

    let zip_avail = has_powershell_zip();
    methods.push(CompressionMethod {
        id: "zip".into(),
        name: if cfg!(windows) { "ZIP (PowerShell)" } else { "ZIP (7z)" }.into(),
        extension: ".zip".into(),
        available: zip_avail || has_binary("7z"),
        supports_password: has_binary("7z"),
        download_url: "https://www.7-zip.org/download.html".into(),
        default_level: 5,
        max_level: 9,
    });

    let has_7z = has_binary("7z");
    methods.push(CompressionMethod {
        id: "7z".into(),
        name: "7-Zip".into(),
        extension: ".7z".into(),
        available: has_7z,
        supports_password: true,
        download_url: "https://www.7-zip.org/download.html".into(),
        default_level: 5,
        max_level: 9,
    });

    let has_zstd = has_binary("zstd");
    methods.push(CompressionMethod {
        id: "zstd".into(),
        name: "Zstandard (zstd)".into(),
        extension: ".zst".into(),
        available: has_zstd,
        supports_password: false,
        download_url: "https://github.com/facebook/zstd/releases".into(),
        default_level: 3,
        max_level: 19,
    });

    let has_gzip = has_binary("gzip");
    methods.push(CompressionMethod {
        id: "gzip".into(),
        name: "Gzip".into(),
        extension: ".gz".into(),
        available: has_gzip,
        supports_password: false,
        download_url: "https://www.gnu.org/software/gzip/".into(),
        default_level: 6,
        max_level: 9,
    });

    let has_xz = has_binary("xz");
    methods.push(CompressionMethod {
        id: "xz".into(),
        name: "XZ".into(),
        extension: ".xz".into(),
        available: has_xz,
        supports_password: false,
        download_url: "https://tukaani.org/xz/".into(),
        default_level: 6,
        max_level: 9,
    });

    let has_lz4 = has_binary("lz4");
    methods.push(CompressionMethod {
        id: "lz4".into(),
        name: "LZ4".into(),
        extension: ".lz4".into(),
        available: has_lz4,
        supports_password: false,
        download_url: "https://lz4.github.io/lz4/".into(),
        default_level: 1,
        max_level: 12,
    });

    methods
}

pub fn compress_file(src: &Path, dst: &Path, method: &str, level: u32, password: Option<&str>) -> Result<(), String> {
    match method {
        "zip" => compress_zip(src, dst, level, password),
        "7z" => compress_7z(src, dst, level, password),
        "zstd" => compress_zstd(src, dst, level),
        "gzip" => compress_gzip(src, dst, level),
        "xz" => compress_xz(src, dst, level),
        "lz4" => compress_lz4(src, dst, level),
        _ => Err(format!("Méthode de compression inconnue : {method}")),
    }
}

fn compress_zip(src: &Path, dst: &Path, _level: u32, password: Option<&str>) -> Result<(), String> {
    if let Some(pw) = password {
        return compress_7z(src, dst, _level, Some(pw));
    }
    if cfg!(windows) {
        let src_str = src.to_string_lossy();
        let dst_str = dst.to_string_lossy();
        let ps_script = format!(
            "Compress-Archive -LiteralPath '{}' -DestinationPath '{}' -CompressionLevel Optimal -Force",
            src_str.replace('\'', "''"),
            dst_str.replace('\'', "''")
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .output()
            .map_err(|e| format!("Échec PowerShell : {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("Compress-Archive échec : {stderr}"));
        }
        Ok(())
    } else {
        compress_7z(src, dst, 5, None)
    }
}

fn compress_7z(src: &Path, dst: &Path, level: u32, password: Option<&str>) -> Result<(), String> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    let mut args = vec!["a".to_string(), "-t7z".to_string(), format!("-mx={}", level), "-y".to_string()];
    if let Some(pw) = password {
        args.push(format!("-p{pw}"));
    }
    args.push(dst_str);
    args.push(src_str);

    let out = Command::new("7z")
        .args(&args)
        .output()
        .map_err(|e| format!("7z introuvable : {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("7z échec : {stderr}"));
    }
    Ok(())
}

fn compress_zstd(src: &Path, dst: &Path, level: u32) -> Result<(), String> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    let out = Command::new("zstd")
        .args([format!("-{level}"), "-f".to_string(), "-o".to_string(), dst_str, src_str])
        .output()
        .map_err(|e| format!("zstd introuvable : {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("zstd échec : {stderr}"));
    }
    Ok(())
}

fn compress_gzip(src: &Path, dst: &Path, level: u32) -> Result<(), String> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    let out = Command::new("gzip")
        .arg(format!("-{level}"))
        .arg("-c")
        .arg(&src_str)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("gzip introuvable : {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("gzip échec : {stderr}"));
    }
    std::fs::write(dst, &out.stdout).map_err(|e| format!("Écriture {dst_str} échec : {e}"))
}

fn compress_xz(src: &Path, dst: &Path, level: u32) -> Result<(), String> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    let out = Command::new("xz")
        .arg(format!("-{level}"))
        .arg("-c")
        .arg(&src_str)
        .stdout(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("xz introuvable : {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("xz échec : {stderr}"));
    }
    std::fs::write(dst, &out.stdout).map_err(|e| format!("Écriture {dst_str} échec : {e}"))
}

fn compress_lz4(src: &Path, dst: &Path, level: u32) -> Result<(), String> {
    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();
    let out = Command::new("lz4")
        .arg(format!("-{level}"))
        .arg("-c")
        .arg(&src_str)
        .stdout(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("lz4 introuvable : {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("lz4 échec : {stderr}"));
    }
    std::fs::write(dst, &out.stdout).map_err(|e| format!("Écriture {dst_str} échec : {e}"))
}

/// Copy mtime from source to destination file for change detection purposes.
pub fn copy_mtime(src: &Path, dst: &Path) -> Result<(), String> {
    let md = std::fs::metadata(src).map_err(|e| format!("mtime src : {e}"))?;
    let mtime = filetime::FileTime::from_last_modification_time(&md);
    filetime::set_file_mtime(dst, mtime).map_err(|e| format!("mtime dst : {e}"))
}
