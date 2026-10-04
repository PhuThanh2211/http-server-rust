use std::collections::HashMap;
use std::path::PathBuf;
use crate::request::Request;

pub fn request(method: &str, path: &str) -> Request {
    Request {
        method: method.to_string(),
        path: path.to_string(),
        version: "HTTP/1.1".to_string(),
        headers: HashMap::new(),
        body: Vec::new(),
        query: Vec::new(),
        host_count: 0,
    }
}

pub fn with_header(mut req: Request, name: &str, value: &str) -> Request {
    req.headers.insert(name.to_ascii_lowercase(), value.to_string());
    req
}

/// A unique temp dir per test, so tests running in parallel don't share files.
pub fn temp_dir_with_file(test_name: &str, file: &str, contents: &[u8]) -> String {
    let dir: PathBuf = std::env::temp_dir().join(format!("http_server_test_{}", test_name));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(file), contents).unwrap();
    dir.to_string_lossy().to_string()
}