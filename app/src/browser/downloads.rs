// Helpers are consumed by `browser/webview_host.rs`, which is itself
// scaffolding for the in-progress download pipeline. Until that pipeline
// gets wired up to a real wry handler, the call chain is unreachable from
// production code paths, so allow dead_code at the module level.
#![allow(dead_code)]

//! Download destination resolution for the embedded browser pane.
//!
//! wry exposes `with_download_started_handler` (a `Fn(url, &mut PathBuf)
//! -> bool` that fills in the destination and gates the download). This
//! module contains the pure decision logic so the wry-side wiring stays
//! a thin shim and the collision-suffix path can be tested without a
//! real WebKit context.
//!
//! ## Behavior
//!
//! - Destination is `~/Downloads` (macOS / Linux conventional) with a
//!   collision suffix: `report.pdf` → `report (1).pdf` →
//!   `report (2).pdf`. Up to `MAX_COLLISION_SUFFIX` attempts; if even
//!   that many already exist we fall through to a millisecond-timestamp
//!   suffix to guarantee uniqueness without rewriting the user's
//!   destination repeatedly.
//! - Source-suggested filename takes precedence; if missing, derive
//!   from the URL path; if that's missing too, fall back to
//!   `download.bin`.
//! - The base dir is created on demand. If creation fails (read-only
//!   FS, permission denied, etc.) we return `None` so the caller can
//!   cancel instead of falling through to wry defaults.
//!
//! ## What this does NOT do
//!
//! - **No MIME / Content-Disposition parsing.** wry hands us the already-
//!   suggested filename (or a synthesized one). We don't re-sniff.
//! - **No safe-file filtering.** Browsers like Chrome warn on `.exe`,
//!   `.dmg`, etc. from untrusted origins. CastCodes accepts any
//!   suggested extension today; gating that is a follow-up.
//! - **No download manager UI.** Surfacing in-pane chrome was listed
//!   alongside this in the audit plan; deferred to keep this PR focused
//!   on the security/correctness baseline (deterministic destination,
//!   no overwrites). Today the user just gets a file in ~/Downloads.

use std::path::{Path, PathBuf};

use url::Url;

/// Hard cap on collision-suffix attempts before we fall back to a
/// timestamp suffix. 100 is more than enough for any reasonable user
/// but cheap to scan.
const MAX_COLLISION_SUFFIX: u32 = 100;

/// Resolve the on-disk destination for a download.
///
/// `base` is the directory downloads go into (typically `~/Downloads`).
/// `suggested` is the filename wry hands us (the suggested filename
/// from the source response, or a synthesized one). Returns an
/// absolute path with a collision suffix if needed.
///
/// The caller is responsible for verifying `base` exists on disk before
/// calling; this function only checks existence to apply the suffix.
pub(crate) fn resolve_destination(base: &Path, suggested: &str) -> PathBuf {
    let name = sanitize_filename(suggested);
    let candidate = base.join(&name);
    if !candidate.exists() {
        return candidate;
    }

    let (stem, ext) = split_stem_ext(&name);
    for n in 1..=MAX_COLLISION_SUFFIX {
        let with_suffix = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        let candidate = base.join(&with_suffix);
        if !candidate.exists() {
            return candidate;
        }
    }

    // Fallback: timestamp suffix. Last-resort uniqueness guarantee for
    // the absurd case where 100 collision-suffixed copies already
    // exist; downloads will keep working rather than silently overwrite.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let with_ts = if ext.is_empty() {
        format!("{stem}-{ts}")
    } else {
        format!("{stem}-{ts}.{ext}")
    };
    base.join(with_ts)
}

/// Strip path separators and other characters that would let a remote
/// site escape `base`. Replaces them with `_`. Empty / dotty names
/// become `download.bin`.
fn sanitize_filename(input: &str) -> String {
    // Just take the file basename — drops any leading directories from
    // the suggestion (defensive against a malicious server returning
    // `Content-Disposition: filename="../../../../etc/passwd"`).
    let basename = Path::new(input)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    sanitize_filename_component(&basename)
}

fn sanitize_filename_component(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .map(|c| match c {
            '/' | '\\' | '\0' | ':' | '<' | '>' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();

    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() || trimmed.chars().all(|c| c == '.') {
        "download.bin".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Split `name` into `(stem, extension)`. The stem is the part before
/// the LAST dot; the extension excludes the dot. Filenames with no dot
/// or starting with a dot return `(name, "")`.
fn split_stem_ext(name: &str) -> (String, String) {
    // Files like `.bashrc` have no extension — the leading dot is part
    // of the stem. Use `rfind` of '.' and skip the leading-only case.
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => {
            (name[..i].to_string(), name[i + 1..].to_string())
        }
        _ => (name.to_string(), String::new()),
    }
}

/// Default downloads directory: `dirs::download_dir()` if available
/// (typically `~/Downloads` on macOS/Linux), else the user's home dir,
/// else `None`. Creates the directory if missing.
pub(crate) fn default_base_dir() -> Option<PathBuf> {
    let dir = dirs::download_dir().or_else(dirs::home_dir)?;
    if let Err(err) = std::fs::create_dir_all(&dir) {
        log::warn!("failed to ensure downloads directory {dir:?}: {err}; cancelling download");
        return None;
    }
    Some(dir)
}

/// Derive a filename from a download URL when wry's suggestion is empty.
/// Parses the URL and decodes the path's last segment; returns
/// `download.bin` if there's nothing useful to use.
pub(crate) fn filename_from_url(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return "download.bin".to_string();
    };
    let Some(last_segment) = parsed
        .path_segments()
        .and_then(|segments| segments.rev().find(|segment| !segment.is_empty()))
    else {
        return "download.bin".to_string();
    };

    sanitize_filename_component(&percent_decode_lossy(last_segment))
}

/// URL form safe enough for app logs: keep origin/path context, drop
/// query/fragment/userinfo values that commonly carry tokens.
pub(crate) fn url_for_log(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        let end = url.find(['?', '#']).unwrap_or(url.len());
        return url[..end].to_string();
    };

    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    parsed.to_string()
}

fn percent_decode_lossy(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                decoded.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sanitize_keeps_safe_filename() {
        assert_eq!(sanitize_filename("report.pdf"), "report.pdf");
        assert_eq!(sanitize_filename("hello world.txt"), "hello world.txt");
        assert_eq!(
            sanitize_filename("file_with-many.chars-2024.tar.gz"),
            "file_with-many.chars-2024.tar.gz"
        );
    }

    #[test]
    fn sanitize_strips_path_traversal() {
        assert_eq!(sanitize_filename("../../../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("/etc/shadow"), "shadow");
        assert_eq!(sanitize_filename("foo/../bar.txt"), "bar.txt");
    }

    #[test]
    fn sanitize_neutralizes_special_chars() {
        assert_eq!(sanitize_filename("ab|cd<ef>gh?ij*kl"), "ab_cd_ef_gh_ij_kl");
        assert_eq!(sanitize_filename("file:name.txt"), "file_name.txt");
        // Control characters → underscore.
        assert_eq!(sanitize_filename("a\u{0001}b"), "a_b");
    }

    #[test]
    fn sanitize_falls_back_for_empty_input() {
        assert_eq!(sanitize_filename(""), "download.bin");
        assert_eq!(sanitize_filename("   "), "download.bin");
        assert_eq!(sanitize_filename("..."), "download.bin");
        assert_eq!(sanitize_filename("/"), "download.bin");
    }

    #[test]
    fn sanitize_preserves_dotfiles() {
        assert_eq!(sanitize_filename(".bashrc"), ".bashrc");
        assert_eq!(sanitize_filename("..hidden"), "..hidden");
        assert_eq!(sanitize_filename(".config.json"), ".config.json");
    }

    #[test]
    fn split_stem_ext_normal() {
        assert_eq!(
            split_stem_ext("report.pdf"),
            ("report".to_string(), "pdf".to_string())
        );
        // Compound extensions: split on the LAST dot.
        assert_eq!(
            split_stem_ext("archive.tar.gz"),
            ("archive.tar".to_string(), "gz".to_string())
        );
    }

    #[test]
    fn split_stem_ext_dotfile() {
        // `.bashrc` has no extension; the leading dot is part of the stem.
        assert_eq!(
            split_stem_ext(".bashrc"),
            (".bashrc".to_string(), "".to_string())
        );
    }

    #[test]
    fn split_stem_ext_no_extension() {
        assert_eq!(
            split_stem_ext("README"),
            ("README".to_string(), "".to_string())
        );
    }

    #[test]
    fn split_stem_ext_trailing_dot() {
        // `foo.` is treated as no extension since there's nothing after.
        assert_eq!(split_stem_ext("foo."), ("foo.".to_string(), "".to_string()));
    }

    #[test]
    fn resolve_destination_uses_suggestion_when_no_collision() {
        let dir = TempDir::new().unwrap();
        let path = resolve_destination(dir.path(), "report.pdf");
        assert_eq!(path, dir.path().join("report.pdf"));
    }

    #[test]
    fn resolve_destination_applies_collision_suffix() {
        let dir = TempDir::new().unwrap();
        // Create the original to force a collision.
        std::fs::write(dir.path().join("report.pdf"), b"x").unwrap();
        let path = resolve_destination(dir.path(), "report.pdf");
        assert_eq!(path, dir.path().join("report (1).pdf"));

        // Create that one too; should now resolve to (2).
        std::fs::write(&path, b"x").unwrap();
        let path = resolve_destination(dir.path(), "report.pdf");
        assert_eq!(path, dir.path().join("report (2).pdf"));
    }

    #[test]
    fn resolve_destination_suffix_works_without_extension() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("README"), b"x").unwrap();
        let path = resolve_destination(dir.path(), "README");
        assert_eq!(path, dir.path().join("README (1)"));
    }

    #[test]
    fn resolve_destination_sanitizes_traversal() {
        let dir = TempDir::new().unwrap();
        let path = resolve_destination(dir.path(), "../../etc/passwd");
        assert_eq!(path, dir.path().join("passwd"));
        assert!(
            path.starts_with(dir.path()),
            "resolved path escaped base: {path:?}"
        );
    }

    #[test]
    fn resolve_destination_empty_suggestion_falls_back() {
        let dir = TempDir::new().unwrap();
        let path = resolve_destination(dir.path(), "");
        assert_eq!(path, dir.path().join("download.bin"));
    }

    #[test]
    fn filename_from_url_uses_last_segment() {
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf"),
            "report.pdf"
        );
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf?token=abc"),
            "report.pdf"
        );
        assert_eq!(
            filename_from_url("https://example.com/files/report.pdf#frag"),
            "report.pdf"
        );
    }

    #[test]
    fn filename_from_url_decodes_last_path_segment() {
        assert_eq!(
            filename_from_url("https://example.com/files/monthly%20report.pdf"),
            "monthly report.pdf"
        );
        assert_eq!(
            filename_from_url("https://example.com/files/a%2Fb.txt"),
            "a_b.txt"
        );
    }

    #[test]
    fn filename_from_url_falls_back_on_empty_path() {
        assert_eq!(filename_from_url("https://example.com/"), "download.bin");
        assert_eq!(filename_from_url("https://example.com"), "download.bin");
        assert_eq!(filename_from_url("not a url"), "download.bin");
    }

    #[test]
    fn url_for_log_strips_sensitive_url_parts() {
        assert_eq!(
            url_for_log("https://user:secret@example.com/files/report.pdf?token=abc#frag"),
            "https://example.com/files/report.pdf"
        );
        assert_eq!(url_for_log("not a url?token=abc#frag"), "not a url");
    }
}
