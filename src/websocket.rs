use base64::{Engine, engine::general_purpose::STANDARD};
use sha1::{Digest, Sha1};

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// base64(sha1(key + GUID)); the digest is the raw 20 bytes, not hex.
pub fn accept_key(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    STANDARD.encode(hasher.finalize())
}

/// Some(accept) when the handshake is valid, None when it must be rejected with 400.
pub fn validate_handshake(
    method: &str,
    upgrade: &str,
    connection: &str,
    key: &str,
    version: &str,
) -> Option<String> {
    let key = key.trim();
    let valid = method == "GET"
        && upgrade.trim().eq_ignore_ascii_case("websocket")
        && connection.to_ascii_lowercase().contains("upgrade")
        && !key.is_empty()
        && version.trim() == "13";

    valid.then(|| accept_key(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_6455_example() {
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn valid_handshake() {
        assert_eq!(
            validate_handshake("GET", "WebSocket", "keep-alive, Upgrade", "dGhlIHNhbXBsZSBub25jZQ==", "13"),
            Some("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=".to_string())
        );
    }

    #[test]
    fn rejects_each_invalid_part() {
        let k = "abc";
        assert_eq!(validate_handshake("POST", "websocket", "Upgrade", k, "13"), None);
        assert_eq!(validate_handshake("GET", "", "Upgrade", k, "13"), None);
        assert_eq!(validate_handshake("GET", "h2c", "Upgrade", k, "13"), None);
        assert_eq!(validate_handshake("GET", "websocket", "keep-alive", k, "13"), None);
        assert_eq!(validate_handshake("GET", "websocket", "Upgrade", "", "13"), None);
        assert_eq!(validate_handshake("GET", "websocket", "Upgrade", k, "8"), None);
    }
}