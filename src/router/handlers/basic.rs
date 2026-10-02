use std::io::Write;
use flate2::Compression;
use flate2::write::GzEncoder;
use crate::{request::Request, response};

pub(crate) fn index(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"index")
}

pub(crate) fn about(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"about")
}

pub(crate) fn login(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"login")
}

pub(crate) fn list_users(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"list_users")
}

pub(crate) fn user_agent(req: &Request, _dir: &str) -> Vec<u8> {
    let ua = req.header("user-agent").unwrap_or("");
    response::ok("text/plain", ua.as_bytes())
}

pub(crate) fn echo(text: &str, req: &Request) -> Vec<u8> {
    let support_gzip = req.header("accept-encoding")
        .map(|v| v.split(',').any(|s| s.trim() == "gzip"))
        .unwrap_or(false);

    if support_gzip {
        let compressed = gzip_compress(text.as_bytes());
        response::ok_with_encoding("text/plain", &compressed, Some("gzip"))
    } else {
        response::ok_with_encoding("text/plain", text.as_bytes(), None)
    }
}
fn gzip_compress(data: &[u8]) -> Vec<u8> {
    // Creates an in-memory gzip writer
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

    // Feeds the raw string bytes through the compressor
    encoder.write_all(data).unwrap();

    // Flushes and returns the complete gzip byte stream (header + compressed data + checksum/trailer)
    encoder.finish().unwrap()
}