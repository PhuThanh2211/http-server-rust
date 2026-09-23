use std::collections::HashMap;
use std::fs::{read, write};
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use flate2::Compression;
use flate2::write::GzEncoder;
use crate::{request::Request, response};

use crate::static_files;

#[derive(Clone)]
enum Segment {
    Literal(String),
    Param(String),
}
type Handler = fn(&Request, &str) -> Vec<u8>;
type ParamHandler = fn(&Request, &str, &HashMap<String, String>) -> Vec<u8>;
struct PatternRoute {
    method: String,
    segments: Vec<Segment>,
    handler: ParamHandler,
}
pub struct Router {
    // path -> method -> handler (path-first structure is what makes 405-vs-404 possible)
    routes: HashMap<String, HashMap<String, Handler>>, // exact matches
    pattern_routes: Vec<PatternRoute>,
}

impl Router {
    fn new() -> Self {
        Router { routes: HashMap::new(), pattern_routes: Vec::new() }
    }

    /// Registers a route. Panics on duplicate (method, path) — a programmer error,
    /// same as real routers failing fast at startup rather than silently shadowing.
    fn register(&mut self, method: &str, path: &str, handler: Handler) {
        let methods = self.routes
            .entry(path.to_string())
            .or_insert_with(HashMap::new);

        if methods.contains_key(method) {
            panic!("Duplicate route registration: {} {}", method, path);
        }

        methods.insert(method.to_string(), handler);
    }

    fn register_pattern(&mut self, method: &str, pattern: &str, handler: ParamHandler) {
        let segments = pattern.trim_start_matches('/').split('/')
            .map(|s| match s.strip_prefix('{').and_then(|x| x.strip_suffix('}')) {
                Some(name) => Segment::Param(name.to_string()),
                None => Segment::Literal(s.to_string()),
            }).collect();

        self.pattern_routes.push(PatternRoute {
            method: method.to_string(),
            segments,
            handler
        })
    }

    fn match_pattern(&self, method: &str, path: &str) -> Option<(ParamHandler, HashMap<String, String>)> {
        let req_segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        for route in &self.pattern_routes {
            if route.method != method || route.segments.len() != req_segments.len() {
                continue;
            }

            let mut params = HashMap::new();
            let mut ok = true;
            for (seq, req_seq)  in route.segments.iter().zip(req_segments.iter()) {
                match seq {
                    Segment::Literal(l) => {
                        if l != req_seq {
                            ok = false;
                            break;
                        }
                    }
                    Segment::Param(name) => {
                        if req_seq.is_empty() {
                            ok = false;
                            break;
                        }
                        params.insert(name.clone(), req_seq.to_string());
                    }
                }
            }

            if ok {
                return Some((route.handler, params));
            }
        }
        None
    }

    fn dispatch(&self, req: &Request, dir: &str) -> Vec<u8> {
        // 1. Static beats dynamic: exact match checked first
        if let Some(methods) = self.routes.get(&req.path) {
            if let Some(handler) = methods.get(req.method.as_str()) {
                return handler(req, dir);
            }

            if req.method == "HEAD" {
                if let Some(get_handler) = methods.get("GET") {
                    return strip_body(get_handler(req, dir));
                }
            }

            let mut allowed: Vec<&str> = methods.keys().map(|s| s.as_str()).collect();
            allowed.sort();
            return response::method_not_allowed(
                "Method not allowed for this resource.\n",
                &allowed.join(", "),
            );
        }

        // 2. Fall back to pattern routes
        if let Some((handler, params)) = self.match_pattern(&req.method, &req.path) {
            return handler(req, dir, &params);
        }

        // 3. Nothing matched
        response::not_found("The requested resource was not found.\n")
    }
}

fn strip_body(response: Vec<u8>) -> Vec<u8> {
    let marker = b"\r\n\r\n";
    match response.windows(4).position(|w| w == marker) {
        Some(pos) => response[..pos + 4].to_vec(), // keep status line + headers onl
        None => response,
    }
}

/// Registered once (startup semantics), reused across all requests.
fn static_router() -> &'static Router {
    static ROUTER: OnceLock<Router> = OnceLock::new();
    ROUTER.get_or_init(|| {
        let mut r = Router::new();
        r.register("GET", "/", handle_index);
        r.register("GET", "/about", handle_about);
        r.register("POST", "/login", handle_login);
        r.register("GET", "/api/users", handle_list_users);
        r.register("GET", "/user-agent", handle_ua_route);
        r.register("GET", "/search", handle_search);

        r.register_pattern("GET", "/users/{id}", handle_get_user);
        r.register_pattern("GET", "/users/{id}/posts/{post}", handle_get_post);
        r
    })
}

pub fn route(req: &Request, dir: &str) -> Vec<u8> {
    if let Some(resp) = try_dynamic_routes(req, dir) {
        return resp;
    }

    static_router().dispatch(req, dir)
}

fn try_dynamic_routes(req: &Request, dir: &str) -> Option<Vec<u8>> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET" , p) if p.starts_with("/echo/") => Some(handle_echo(&p[6..], req)),
        ("GET" , p) if p.starts_with("/files/") => Some(handle_get_file(&p[7..], dir)),
        ("POST" , p) if p.starts_with("/files/") => Some(handle_post_file(&p[7..], dir,
                                                                          &req.body)),
        // Known path, wrong method → 405 with Allow header (not silently 404)
        (_, p) if p.starts_with("/files/") => Some(response::method_not_allowed(
            "This endpoint only supports GET and POST.\n", "GET, POST"
        )),
        _ => None,
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

fn handle_index(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"index")
}

fn handle_about(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"about")
}

fn handle_login(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"login")
}

fn handle_list_users(_req: &Request, _dir: &str) -> Vec<u8> {
    response::ok("text/plain", b"list_users")
}

fn handle_ua_route(req: &Request, _dir: &str) -> Vec<u8> {
    let ua = req.header("user-agent").unwrap_or("");
    response::ok("text/plain", ua.as_bytes())
}

fn handle_get_file(filename: &str, dir: &str) -> Vec<u8> {
    match static_files::resolve_on_disk(dir, filename) {
        Some(path) => match read(&path) {
            Ok(contents) => {
                let mime = static_files::mime_type_for(filename);
                response::ok(mime, &contents)
            }
            Err(_) => response::not_found("The requested file does not exist.\n"),
        }
        None => response::forbidden("Path escapes the allowed directory.\n"),
    }

}

fn handle_post_file(filename: &str, dir: &str, body: &[u8]) -> Vec<u8> {
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

fn handle_get_user(_req: &Request, _dir: &str, params: &HashMap<String, String>) -> Vec<u8> {
    response::ok("text/plain", format_params("get_user", params).as_bytes())
}

fn handle_get_post(_req: &Request, _dir: &str, params: &HashMap<String, String>) -> Vec<u8> {
    response::ok("text/plain", format_params("get_post", params).as_bytes())
}

fn handle_search(req: &Request, _dir: &str) -> Vec<u8> {
    let mut parts = vec!["search".to_string()];

    for (k, v) in &req.query {
        parts.push(format!("{}={}", k, v));
    }

    response::ok("text/plain", parts.join(" ").as_bytes())
}
fn format_params(name: &str, params: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = params.keys().collect();
    keys.sort();
    let mut parts = vec![name.to_string()];

    for k in keys {
        parts.push(format!("{}={}", k, params[k]));
    }
    parts.join(" ")
}