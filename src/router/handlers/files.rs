use std::fs::{read, write};
use crate::{request::Request, response, static_files};

pub(crate) fn get(filename: &str, dir: &str, req: &Request) -> Vec<u8> {
    match static_files::resolve_on_disk(dir, filename) {
        Some(path) => match read(&path) {
            Ok(contents) => {
                let etag = static_files::etag_for(&contents);
                let if_none_match = req.header("if-none-match").unwrap_or("");

                if static_files::should_return_304(&etag, if_none_match) {
                    return response::not_modified(&etag);
                }

                let mime = static_files::mime_type_for(filename);
                let resp = response::ok(mime, &contents);
                response::add_header(resp, &format!("Etag: {}", etag))
            }
            Err(_) => response::not_found("The requested file does not exist.\n"),
        }
        None => response::forbidden("Path escapes the allowed directory.\n"),
    }

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
}