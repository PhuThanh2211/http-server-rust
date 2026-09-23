use std::collections::HashMap;
use std::io::Read;

pub struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub query: Vec<(String, String)>, // preserves original left-to-right order; duplicates allowed
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(|s| s.as_str())
    }

    /// Duplicate-key policy: last occurrence wins.
    pub fn query_get(&self, name: &str) -> Option<&str> {
        self.query.iter().rev()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

pub fn parse_request<T: Read>(stream: &mut T) -> Option<Request> {
    // 1. Initial read (headers + possibly some/all of the body)
    let mut buf = [0u8; 1024];
    let bytes_read = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return None,
    };

    // Use lossy conversion only for parsing headers (text-safe);
    // raw bytes are used separately for the body (binary-safe).
    let head_str = String::from_utf8_lossy(&buf[..bytes_read]);

    // Pretty-printed request
    let lines: Vec<&str> = head_str.lines().collect();
    println!("Request lines: {:#?}", lines);

    // 2. Parse request line: "GET /users/alice/posts/abc-123 HTTP/1.1"
    // "GET /search?q=cron&limit=10 HTTP/1.1"
    let first_line = head_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let raw_path = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("HTTP/1.1").to_string();

    let mut path_and_query = raw_path.splitn(2, '?');
    let path = path_and_query.next().unwrap_or("").to_string();
    let query_str = path_and_query.next().unwrap_or("");
    let query = parse_query(query_str);

    // 3. Parse headers into a HashMap (lowercase keys for case-insensitivity)
    let mut headers = HashMap::new();
    for line in head_str.lines().skip(1) {
        if line.is_empty() {
            break; // blank line marks end of headers
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(
                key.trim().to_ascii_lowercase(),
                value.trim().to_string()
            );
        }
    }

    // 4. Determine Content-Length (0 if absent)
    let content_length: usize = headers.get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // 5. Find where headers end in the raw byte buffer (binary-safe slicing)
    let header_end = head_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(bytes_read);
    let mut body = buf[header_end..bytes_read].to_vec();

    // 6. Keep reading from the socket until the full body has arrived
    while body.len() < content_length {
        let mut chunk = [0u8; 1024];
        match stream.read(&mut chunk) {
            Ok(0) => break, // connection closed early
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }

    Some(Request {
        method, path, version, headers, body, query
    })
}

fn parse_query(qs: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if qs.is_empty() {
        return pairs;
    }

    for pair in qs.split('&') {
        if pair.is_empty() {
            continue;
        }

        let mut kv = pair.splitn(2, '=');
        let raw_key = kv.next().unwrap_or("");
        let raw_val = kv.next().unwrap_or("");
        pairs.push((percent_decode(raw_key), percent_decode(raw_val)));
    }

    pairs
}

fn percent_decode(s: &str) -> String {
    // q=hello%20world → hello world
    // + also means space in query strings
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            },
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        // malformed %XX, keep literal
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).to_string()
}