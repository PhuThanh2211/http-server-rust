use chrono::Utc;

pub struct LogEntry<'a> {
    pub ip: String,
    pub method: &'a str,
    pub path: &'a str,
    pub version: &'a str,
    pub status: u16,
    pub bytes: usize,
    pub user_agent: &'a str,
    pub latency_ms: u128,
}

pub fn log_request(entry: &LogEntry) {
    // Input: <ip>|<timestamp_iso>|<method>|<path>|<version>|<status>|<bytes>|<user_agent>|<latency_ms>
    // Output: <ip> - - [<timestamp_clf>] "<method> <path> <version>" <status> <bytes> "-" "<ua>" <latency>ms

    let timestamp = Utc::now().format("%d/%b/%Y:%H:%M:%S +0000").to_string();
    println!(
        "{} - - [{}] \"{} {} {}\" {} {} \"-\" \"{}\" {}ms",
        entry.ip,
        timestamp,
        entry.method,
        entry.path,
        entry.version,
        entry.status,
        entry.bytes,
        entry.user_agent,
        entry.latency_ms
    );
}

/// Parses "HTTP/1.1 200 OK\r\n..." -> 200
pub fn extract_status(response: &[u8]) -> u16 {
    let head = String::from_utf8_lossy(response);
    head.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0)
}

/// Parses the Content-Length header value (falls back to 0 if absent/invalid)
pub fn extract_body_length(response: &[u8]) -> usize {
    let head = String::from_utf8_lossy(response);
    head.lines()
        .find_map(|line| line.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case("content-length")))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0)
}