use std::fs::{read, write};
use std::io::Write;
use std::path::Path;
use flate2::Compression;
use flate2::write::GzEncoder;
use crate::{request::Request, response};

pub fn route(req: &Request, dir: &str) -> Vec<u8> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET" , "/") => response::ok("text/plain", b""),
        ("GET" , p) if p.starts_with("/echo/") => handle_echo(&p[6..], req),
        ("GET" , "/user-agent") => handle_user_agent(req),
        ("GET" , p) if p.starts_with("/files/") => handle_get_file(&p[7..], dir),
        ("POST" , p) if p.starts_with("/files/") => handle_post_file(&p[7..], dir, &req.body),
        _ => response::not_found(),
    }
}

fn handle_echo(text: &str, req: &Request) -> Vec<u8> {
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

fn handle_user_agent(req: &Request) -> Vec<u8> {
    let ua = req.header("user-agent").unwrap_or("");
    response::ok("text/plain", ua.as_bytes())
}

fn handle_get_file(filename: &str, dir: &str) -> Vec<u8> {
    match read(Path::new(dir).join(filename)) {
        Ok(contents) => response::ok("application/octet-stream", &contents),
        Err(_) => response::not_found(),
    }
}

fn handle_post_file(filename: &str, dir: &str, body: &[u8]) -> Vec<u8> {
    match write(Path::new(dir).join(filename), body) {
        Ok(_contents) => response::created(),
        Err(_) => response::server_error(),
    }
}