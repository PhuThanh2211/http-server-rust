use std::collections::HashMap;
use std::io::Read;

pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(|s| s.as_str())
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

    // 2. Parse request line: "GET /path HTTP/1.1"
    let first_line = head_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

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
        method, path, headers, body
    })
}