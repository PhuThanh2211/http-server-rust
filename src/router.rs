use std::collections::HashMap;
use std::sync::OnceLock;
use crate::{request::Request, response};

mod handlers;
use handlers::{basic, files, users};

#[cfg(test)]
mod tests;

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
        r.register("GET", "/", basic::index);
        r.register("GET", "/about", basic::about);
        r.register("POST", "/login", basic::login);
        r.register("GET", "/api/users", basic::list_users);
        r.register("GET", "/user-agent", basic::user_agent);
        r.register("GET", "/search", users::search);
        r.register("GET", "/crash", |_req, _dir| panic!("simulated bug"));

        r.register_pattern("GET", "/users/{id}", users::get_user);
        r.register_pattern("GET", "/users/{id}/posts/{post}", users::get_post);
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
        ("GET" , p) if p.starts_with("/echo/") => Some(basic::echo(&p[6..], req)),
        ("GET" , p) if p.starts_with("/files/") => Some(files::get(&p[7..], dir, req)),
        ("POST" , p) if p.starts_with("/files/") => Some(files::post(&p[7..], dir,
                                                                          &req.body)),
        // Known path, wrong method → 405 with Allow header (not silently 404)
        (_, p) if p.starts_with("/files/") => Some(response::method_not_allowed(
            "This endpoint only supports GET and POST.\n", "GET, POST"
        )),
        _ => None,
    }
}