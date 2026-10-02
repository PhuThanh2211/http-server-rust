use std::collections::HashMap;
use crate::request::Request;
use crate::response;

pub(crate) fn get_user(_req: &Request, _dir: &str, params: &HashMap<String, String>) -> Vec<u8> {
    response::ok("text/plain", format_params("get_user", params).as_bytes())
}

pub(crate) fn get_post(_req: &Request, _dir: &str, params: &HashMap<String, String>) -> Vec<u8> {
    response::ok("text/plain", format_params("get_post", params).as_bytes())
}

pub(crate) fn search(req: &Request, _dir: &str) -> Vec<u8> {
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