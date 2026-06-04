use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Crée une Command qui n'ouvre PAS de fenêtre console sous Windows (CREATE_NO_WINDOW) :
/// évite le flash de terminal au démarrage (détection des outils) et pendant la compression.
fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    c.creation_flags(0x0800_0000);
    c
}

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
    /// Compression indicative : Aucune / Moyenne / Élevée / Très élevée / Ultra.
    pub ratio: String,
    /// true = intégré à l'app (rien à installer) ; false = outil externe.
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionConfig {
    pub method: String,
    pub level: u32,
    pub password: Option<String>,
    #[serde(default)]
    pub archive_name: String,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        CompressionConfig {
            method: "off".into(),
            level: 0,
            password: None,
            archive_name: String::new(),
        }
    }
}

/// INVARIANT SÉCURITÉ : `name` doit toujours provenir d'une constante codée en dur
/// (les ids de `detect_methods` : "zpaq"/"7z"/"rar"/"tar"). Ne JAMAIS passer ici un nom
/// issu de la config, du réseau ou d'une saisie : ce serait exécuter un binaire arbitraire
/// du PATH. Si la détection devient un jour dynamique, valider contre une liste blanche.
fn has_binary(name: &str) -> bool {
    debug_assert!(
        matches!(name, "zpaq" | "7z" | "rar" | "tar"),
        "has_binary appelé avec un nom non codé en dur : {name}"
    );
    command(name).arg("--help").output().is_ok()
}

/// Formats capables de chiffrer l'archive par mot de passe :
/// - ZIP intégré (deflate/bzip2/zstd) via AES-256 du conteneur ZIP ;
/// - 7-Zip et RAR via leur chiffrement natif (AES-256).
/// tar.* (pas de chiffrement) et zpaq ne gèrent pas de mot de passe.
pub fn supports_password(id: &str) -> bool {
    matches!(id, "deflate" | "bzip2" | "zstd" | "7z" | "rar")
}

#[allow(clippy::too_many_arguments)]
fn method(
    id: &str,
    name: &str,
    ext: &str,
    ratio: &str,
    default_level: u32,
    max_level: u32,
    available: bool,
    builtin: bool,
    url: &str,
) -> CompressionMethod {
    CompressionMethod {
        id: id.into(),
        name: name.into(),
        extension: ext.into(),
        available,
        supports_password: supports_password(id),
        download_url: url.into(),
        default_level,
        max_level,
        ratio: ratio.into(),
        builtin,
    }
}

/// Codecs internes du ZIP (toujours dispo, mise à jour incrémentale) + formats
/// externes (reconstruction complète, nécessitent l'outil installé).
pub fn detect_methods() -> Vec<CompressionMethod> {
    let tar = has_binary("tar");
    // Ordonné de la compression la plus forte à la plus légère.
    vec![
        method("zpaq", "zpaq (.zpaq, maximum absolu)", ".zpaq", "Maximum", 5, 5, has_binary("zpaq"), false, "http://mattmahoney.net/dc/zpaq.html"),
        method("7z", "7-Zip (.7z)", ".7z", "Ultra", 5, 9, has_binary("7z"), false, "https://www.7-zip.org/download.html"),
        method("tar.xz", "tar + XZ (.tar.xz)", ".tar.xz", "Ultra", 0, 0, tar, false, "https://tukaani.org/xz/"),
        method("rar", "WinRAR (.rar)", ".rar", "Ultra", 3, 5, has_binary("rar"), false, "https://www.win-rar.com/download.html"),
        method("zstd", "Zstandard (ZIP intégré)", ".zip", "Très élevée", 10, 22, true, true, ""),
        method("tar.zst", "tar + Zstandard (.tar.zst)", ".tar.zst", "Très élevée", 0, 0, tar, false, "https://github.com/facebook/zstd/releases"),
        method("bzip2", "Bzip2 (ZIP intégré)", ".zip", "Élevée", 9, 9, true, true, ""),
        method("tar.bz2", "tar + Bzip2 (.tar.bz2)", ".tar.bz2", "Élevée", 0, 0, tar, false, "https://www.gnu.org/software/tar/"),
        method("deflate", "Deflate (ZIP intégré)", ".zip", "Moyenne", 6, 9, true, true, ""),
        method("tar.gz", "tar + Gzip (.tar.gz)", ".tar.gz", "Moyenne", 0, 0, tar, false, "https://www.gnu.org/software/tar/"),
        method("tar.lz4", "tar + LZ4 (.tar.lz4, rapide)", ".tar.lz4", "Faible", 0, 0, tar, false, "https://github.com/lz4/lz4/releases"),
    ]
}

/// Codec interne au conteneur ZIP (chemin incrémental en Rust).
pub fn is_builtin(method: &str) -> bool {
    matches!(method, "deflate" | "bzip2" | "zstd")
}

pub fn archive_extension(method: &str) -> &'static str {
    match method {
        "zpaq" => ".zpaq",
        "7z" => ".7z",
        "rar" => ".rar",
        "tar.zst" => ".tar.zst",
        "tar.xz" => ".tar.xz",
        "tar.gz" => ".tar.gz",
        "tar.bz2" => ".tar.bz2",
        "tar.lz4" => ".tar.lz4",
        _ => ".zip",
    }
}

/// Construit une archive complète via un outil externe (full rebuild).
/// `src_dir` = dossier source, `out` = fichier archive à écrire.
/// `password` (si Some et non vide) chiffre l'archive pour les formats qui le
/// gèrent (7-Zip : `-p` + `-mhe=on` pour chiffrer aussi les noms ; RAR : `-hp`).
/// Note sécurité : le mot de passe est passé en argument de la commande, donc
/// brièvement visible dans la liste des processus (limite des outils externes).
pub fn build_external(
    method: &str,
    level: u32,
    password: Option<&str>,
    src_dir: &Path,
    out: &Path,
) -> Result<(), String> {
    let src = src_dir.to_string_lossy().to_string();
    let outp = out.to_string_lossy().to_string();
    let glob = format!("{src}\\*");
    let pwd = password.map(str::trim).filter(|p| !p.is_empty());
    let result = match method {
        "zpaq" => command("zpaq")
            .args(["a", &outp, &glob, &format!("-m{}", level.clamp(1, 5))])
            .output(),
        "7z" => {
            let mut args = vec![
                "a".to_string(),
                "-t7z".to_string(),
                format!("-mx={}", level.clamp(1, 9)),
                "-y".to_string(),
            ];
            if let Some(p) = pwd {
                args.push(format!("-p{p}"));
                args.push("-mhe=on".to_string());
            }
            args.push(outp.clone());
            args.push(glob.clone());
            command("7z").args(&args).output()
        }
        "rar" => {
            let mut args = vec![
                "a".to_string(),
                "-r".to_string(),
                "-ep1".to_string(),
                format!("-m{}", level.clamp(0, 5)),
                "-y".to_string(),
            ];
            if let Some(p) = pwd {
                args.push(format!("-hp{p}"));
            }
            args.push(outp.clone());
            args.push(glob.clone());
            command("rar").args(&args).output()
        }
        "tar.zst" => command("tar").args(["--zstd", "-cf", &outp, "-C", &src, "."]).output(),
        "tar.xz" => command("tar").args(["-cJf", &outp, "-C", &src, "."]).output(),
        "tar.gz" => command("tar").args(["-czf", &outp, "-C", &src, "."]).output(),
        "tar.bz2" => command("tar").args(["-cjf", &outp, "-C", &src, "."]).output(),
        "tar.lz4" => command("tar").args(["--lz4", "-cf", &outp, "-C", &src, "."]).output(),
        _ => return Err(format!("format externe inconnu : {method}")),
    };
    match result {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!("{method} : {}", String::from_utf8_lossy(&o.stderr).trim())),
        Err(e) => Err(format!("outil « {method} » introuvable : {e}")),
    }
}
