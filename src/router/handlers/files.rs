use std::fs::{read, write};
use crate::{range, request::Request, response, static_files};

pub(crate) fn get(filename: &str, dir: &str, req: &Request) -> Vec<u8> {
    let path = match static_files::resolve_on_disk(dir, filename) {
        Some(p) => p,
        None => return response::forbidden("Path escapes the allowed directory.\n"),
    };
    let contents = match read(&path) {
        Ok(c) => c,
        Err(_) => return response::not_found("The requested file does not exists.\n"),
    };

    let etag = static_files::etag_for(&contents);
    if static_files::should_return_304(&etag, req.header("if-none-match").unwrap_or("")) {
        return response::not_modified(&etag);
    }

    let mime = static_files::mime_type_for(filename);
    let size = contents.len() as u64;

    if let Some(spec) = req.header("range").and_then(|v| v.strip_prefix("bytes=")) {
        if !spec.contains(',') {
            return match range::resolve_range(size, spec) {
                range::RangeOutcome::Slice { start, end } => response::partial_content(
                    mime, &contents[start as usize..=end as usize], start, end, size, &etag
                ),
                range::RangeOutcome::Unsatisfiable => response::range_not_satisfiable(size),
            };
        }
    }

    // Normal 200: advertise that ranges are supported
    let resp = response::ok(mime, &contents);
    let resp = response::add_header(resp, &format!("ETag: {}", etag));
    response::add_header(resp, "Accept-Ranges: bytes")
}

pub(crate) fn post(filename: &str, dir: &str, body: &[u8]) -> Vec<u8> {
    // For a new file, its parent must still resolve safely even though the file itself
    // doesn't exist yet — resolve_safe_path (lexical) covers this; resolve_on_disk would
    // fail since canonicalize requires the file to exist.
    match static_files::resolve_safe_path(dir, filename) {
        Some(path) => match write(&path, body) {
            Ok(_) => response::created(),
            Err(_) => response::internal_server_error("Failed to write file.\n"),
        }
        None => response::forbidden("Path escapes the allowed directory.\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{request, temp_dir_with_file, with_header};

    fn text(bytes: Vec<u8>) -> String {
        String::from_utf8_lossy(&bytes).to_string()
    }

    #[test]
    fn first_request_returns_200_with_etag() {
        let dir = temp_dir_with_file("etag_first", "app.css", b"body{}");
        let etag = static_files::etag_for(b"body{}");
        let resp = text(get("app.css", &dir, &request("GET", "/files/app.css")));
        assert!(resp.starts_with("HTTP/1.1 200"));
        assert!(resp.contains(&format!("ETag: {}", etag)));
        assert!(resp.ends_with("body{}"));
    }

    #[test]
    fn matching_etag_returns_304_without_body() {
        let dir = temp_dir_with_file("etag_match", "app.css", b"body{}");
        let etag = static_files::etag_for(b"body{}");
        let req = with_header(request("GET", "/files/app.css"), "If-None-Match", &etag);
        let resp = text(get("app.css", &dir, &req));
        assert!(resp.starts_with("HTTP/1.1 304"));
        assert!(!resp.to_lowercase().contains("content-length"));
        assert!(resp.ends_with("\r\n\r\n"));
    }

    #[test]
    fn stale_etag_returns_200() {
        let dir = temp_dir_with_file("etag_stale", "app.css", b"body{}");
        let req = with_header(request("GET", "/files/app.css"), "If-None-Match", "\"old\"");
        assert!(text(get("app.css", &dir, &req)).starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn traversal_returns_403() {
        let dir = temp_dir_with_file("traversal", "app.css", b"x");
        let resp = text(get("../../etc/passwd", &dir, &request("GET", "/files/x")));
        assert!(resp.starts_with("HTTP/1.1 403"));
    }

    fn serve(test: &str, range: Option<&str>) -> String {
        let dir = temp_dir_with_file(test, "digits.txt", b"0123456789");
        let mut req = request("GET", "/files/digists.txt");
        if let Some(r) = range {
            req = with_header(req, "Range", r);
        }
        text(get("digits.txt", &dir, &req))
    }

    #[test]
    fn range_returns_206_with_exact_slice() {
        let resp = serve("range_ok", Some("bytes=2-5"));
        assert!(resp.starts_with("HTTP/1.1 206"));
        assert!(resp.contains("Content-Range: bytes 2-5/10"));
        assert!(resp.contains("Content-Length: 4"));
        assert!(resp.ends_with("2345"));
    }

    #[test]
    fn suffix_range_returns_last_bytes() {
        let resp = serve("range_suffix", Some("bytes=-3"));
        assert!(resp.contains("Content-Range: bytes 7-9/10"));
        assert!(resp.ends_with("789"));
    }

    #[test]
    fn start_beyond_file_returns_416_with_size() {
        let resp = serve("range_416", Some("bytes=50-"));
        assert!(resp.starts_with("HTTP/1.1 416"));
        assert!(resp.contains("Content-Range: bytes */10"));
    }

    #[test]
    fn plain_get_advertises_accept_ranges() {
        let resp = serve("range_plain", None);
        assert!(resp.starts_with("HTTP/1.1 200"));
        assert!(resp.contains("Accept-Ranges: bytes"));
    }

    #[test]
    fn multi_range_is_ignored_and_serves_full_file() {
        let resp = serve("range_multi", Some("bytes=0-1,5-6"));
        assert!(resp.starts_with("HTTP/1.1 200"));
        assert!(resp.ends_with("0123456789"));
    }
}