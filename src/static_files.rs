use std::fs::canonicalize;
use std::path::PathBuf;
use sha1::{Digest, Sha1};

// Posix-style lexical normalization: resolves "." and ".." segments without touching the filesystem
fn normalize(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => continue,
            ".." => {
                stack.pop();
            },
            seg => stack.push(seg),
        }
    }

    format!("/{}", stack.join("/"))
}

pub fn mime_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");

    if !path.contains('.') {
        return "application/octet-stream";
    }

    match ext.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub fn resolve_safe_path(root: &str, request_path: &str) -> Option<String> {
    let combined = format!("{}/{}", root.trim_end_matches('/'), request_path.trim_start_matches('/'));
    let normalized_root = normalize(root);
    let normalized_combined = normalize(&combined);

    if normalized_combined == normalized_root
        || normalized_combined.starts_with(&format!("{}/", normalized_root)) {
        Some(normalized_combined)
    } else {
        None
    }
}

pub fn resolve_on_disk(root: &str, request_path: &str) -> Option<PathBuf> {
    // 1. Lexical check on the REQUEST path only, against a fake POSIX root.
    //    Rejects ".." climbs without ever touching a Windows path.
    let safe = resolve_safe_path("/root", request_path)?;
    let relative = safe.trim_start_matches("/root").trim_start_matches('/');

    // 2. Join onto the real root using Path, which understands both separators.
    let canonical_root = canonicalize(root).ok()?;
    let candidate = canonical_root.join(relative);

    // 3. If the file exists, resolve symlinks and re-check containment.
    //    Both values come from canonicalize, so they share the same prefix.
    match canonicalize(&candidate) {
        Ok(real) if real.starts_with(&canonical_root) => Some(real),
        Ok(_) => None,                 // a symlink pointed outside root
        Err(_) => Some(candidate),     // doesn't exist yet (POST, or a 404 on GET)
    }
}

/// Strong ETag: SHA-1 of the bytes, hex-encoded, wrapped in quotes (quotes are part of the ETag).
pub fn etag_for(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    format!("\"{}\"", hex::encode(hasher.finalize()))
}

fn strip_weak(tag: &str) -> &str {
    let t = tag.trim();
    t.strip_prefix("W/").unwrap_or(t)
}

pub fn should_return_304(resource_etag: &str, if_none_match: &str) -> bool {
    let header = if_none_match.trim();
    if header.is_empty() {
        return false;
    }
    if header == "*" {
        return true;
    }
    let current = strip_weak(resource_etag);
    header.split(',').any(|candidate| strip_weak(candidate) == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert_eq!(resolve_safe_path("/var/www", "/../etc/passwd"), None);
    }

    #[test]
    fn allows_interior_climb_that_stays_inside() {
        assert_eq!(
            resolve_safe_path("/var/www", "/a/../index.html"),
            Some("/var/www/index.html".to_string())
        );
    }

    #[test]
    fn rejects_sibling_directory_prefix_bypass() {
        // "/var/www-backup" must NOT be considered "inside" root "/var/www"
        assert_eq!(resolve_safe_path("/var/www", "/../www-backup/x"), None);
    }

    #[test]
    fn unknown_extension_defaults_to_octet_stream() {
        assert_eq!(mime_type_for("/file.xyz"), "application/octet-stream");
    }

    #[test]
    fn etag_is_quoted_and_deterministic() {
        let a = etag_for(b"body{}");
        assert!(a.starts_with('"') && a.ends_with('"'));
        assert_eq!(a, etag_for(b"body{}"));
    }

    #[test]
    fn etag_changes_when_content_changes() {
        assert_ne!(etag_for(b"v1"), etag_for(b"v2"));
    }

    #[test]
    fn etag_has_expected_sha1_length() {
        // 40 hex chars + 2 quotes
        assert_eq!(etag_for(b"x").len(), 42);
    }

    #[test]
    fn conditional_rules() {
        assert!(should_return_304("\"a1b2\"", "\"a1b2\""));
        assert!(!should_return_304("\"a1b2\"", "\"different\""));
        assert!(!should_return_304("\"a1b2\"", ""));
        assert!(should_return_304("\"a1b2\"", "*"));
        assert!(should_return_304("\"a1b2\"", "W/\"a1b2\""));
        assert!(should_return_304("\"a1b2\"", "\"x\", \"y\", \"a1b2\""));
    }
}