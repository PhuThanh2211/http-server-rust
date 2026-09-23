pub fn ok(content_type: &str, body: &[u8]) -> Vec<u8> {
    ok_with_encoding(content_type, body, None)
}

pub fn ok_with_encoding(content_type: &str, body: &[u8], encoding: Option<&str>) -> Vec<u8> {
    let encoding_header = match encoding {
        Some(enc) => format!("Content-Encoding: {}\r\n", enc),
        None => String::new()
    };

    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n{}Content-Length: {}\r\n\r\n",
        content_type,
        encoding_header,
        body.len()
    );
    [header.into_bytes(), body.to_vec()].concat()
}

pub fn add_header(response: Vec<u8>, header_line: &str) -> Vec<u8> {
    // Find the end of headers ("\r\n\r\n") and insert the new header just before it
    let marker = b"\r\n\r\n";
    if let Some(pos) = response.windows(4).position(|w| w == marker) {
        let mut result = response[..pos].to_vec();
        result.extend_from_slice(b"\r\n");
        result.extend_from_slice(header_line.as_bytes());
        result.extend_from_slice(&response[pos..]); // keep "\r\n\r\n" + body

        result
    } else {
        response
    }
}

pub fn error(status_code: u16, reason: &str, body: &str, extra_headers: &[(&str, &str)]) -> Vec<u8> {
    let mut headers = String::new();
    for (name, value) in extra_headers {
        headers.push_str(&format!("{}: {}\r\n", name, value));
    }

    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n{}\r\n",
        status_code,
        reason,
        body.len(),
        headers
    );

    [head.into_bytes(), body.as_bytes().to_vec()].concat()
}

// --- Convenience wrappers (client errors: 4xx) ---
pub fn bad_request(msg: &str) -> Vec<u8> {
    error(400, "Bad Request", msg, &[])
}
pub fn unauthorized(msg: &str, auth_scheme: &str) -> Vec<u8> {
    error(401, "Unauthorized", msg, &[("WWW-Authenticate", auth_scheme)])
}
pub fn forbidden(msg: &str) -> Vec<u8> {
    error(403, "Forbidden", msg, &[])
}
pub fn not_found(msg: &str) -> Vec<u8> {
    error(404, "Not Found", msg, &[])
}
pub fn method_not_allowed(msg: &str, allowed_methods: &str) -> Vec<u8> {
    error(405, "Method Not Allowed", msg, &[("Allow", allowed_methods)])
}
pub fn request_timeout(msg: &str) -> Vec<u8> {
    error(408, "Request Timeout", msg, &[])
}
pub fn conflict(msg: &str) -> Vec<u8> {
    error(409, "Conflict", msg, &[])
}
pub fn payload_too_large(msg: &str) -> Vec<u8> {
    error(413, "Payload Too Large", msg, &[])
}
pub fn uri_too_long(msg: &str) -> Vec<u8> {
    error(414, "URI Too Long", msg, &[])
}
pub fn unsupported_media_type(msg: &str) -> Vec<u8> {
    error(415, "Unsupported Media Type", msg, &[])
}
pub fn too_many_requests(msg: &str) -> Vec<u8> {
    error(429, "Too Many Requests", msg, &[])
}
// --- Convenience wrappers (server errors: 5xx) ---
pub fn internal_server_error(msg: &str) -> Vec<u8> {
    error(500, "Internal Server Error", msg, &[])
}
pub fn bad_gateway(msg: &str) -> Vec<u8> {
    error(502, "Bad Gateway", msg, &[])
}
pub fn service_unavailable(msg: &str) -> Vec<u8> {
    error(503, "Service Unavailable", msg, &[])
}
pub fn gateway_timeout(msg: &str) -> Vec<u8> {
    error(504, "Gateway Timeout", msg, &[])
}

pub fn created() -> Vec<u8> { b"HTTP/1.1 201 Created\r\n\r\n".to_vec() }