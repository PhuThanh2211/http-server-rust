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

pub fn created()-> Vec<u8> { b"HTTP/1.1 201 Created\r\n\r\n".to_vec() }
pub fn not_found() -> Vec<u8> { b"HTTP/1.1 404 Not Found\r\n\r\n".to_vec() }
pub fn server_error() -> Vec<u8> { b"HTTP/1.1 500 Internal Server Error\r\n\r\n".to_vec() }