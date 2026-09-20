use std::fs::{read, write};
use std::path::Path;
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

    let encoding = if support_gzip {
        Some("gzip")
    } else {
        None
    };

    response::ok_with_encoding("text/plain", text.as_bytes(), encoding)
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
        Ok(contents) => response::created(),
        Err(_) => response::server_error(),
    }
}