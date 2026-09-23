use std::fs::canonicalize;
use std::path::PathBuf;

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
    let lexical = resolve_safe_path(root, request_path)?;

    let canonical_root = canonicalize(root).ok()?;
    let candidate = PathBuf::from(&lexical);
    let parent = candidate.parent()?;
    let file_name = candidate.file_name()?;

    let canonical_parent = canonicalize(parent).ok()
        .unwrap_or_else(|| parent.to_path_buf());
    let canonical_candidate = canonical_parent.join(file_name);

    if canonical_candidate.starts_with(&canonical_root) {
        Some(canonical_candidate)
    } else {
        None
    }
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
}