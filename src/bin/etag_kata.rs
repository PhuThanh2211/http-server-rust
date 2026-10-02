use std::io;
use std::io::BufRead;

fn strip_weak(tag: &str) -> &str {
    let t = tag.trim();
    t.strip_prefix("W/").unwrap_or(t)
}

pub fn should_return_304(resource_etag: &str, if_none_match: &str) -> bool {
    let header = if_none_match.trim();

    if header.is_empty() {
        return false;
    }

    if header == "*" {
        return true;
    }

    let current = strip_weak(resource_etag);
    header.split(',').any(|candidate| strip_weak(candidate) == current)
}
fn main() {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        // split at the FIRST '|': left = resource ETag, right = header (may be empty)
        let (resource_etag, if_none_match) = line.split_once('|')
            .unwrap_or((line.as_str(), ""));

        if should_return_304(resource_etag, if_none_match) {
            println!("304");
        } else {
            println!("200");
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_is_304() {
        assert!(should_return_304("\"a1b2\"", "\"a1b2\""));
    }

    #[test]
    fn mismatch_is_200() {
        assert!(!should_return_304("\"a1b2\"", "\"different\""));
    }

    #[test]
    fn empty_header_is_200() {
        assert!(!should_return_304("\"a1b2\"", ""));
        assert!(!should_return_304("\"a1b2\"", "   "));
    }

    #[test]
    fn star_is_304() {
        assert!(should_return_304("\"a1b2\"", "*"));
    }

    #[test]
    fn weak_header_matches_strong_resource() {
        assert!(should_return_304("\"a1b2\"", "W/\"a1b2\""));
    }

    #[test]
    fn weak_resource_matches_strong_header() {
        assert!(should_return_304("W/\"a1b2\"", "\"a1b2\""));
    }

    #[test]
    fn list_with_any_match_is_304() {
        assert!(should_return_304("\"a1b2\"", "\"x\", \"y\", \"a1b2\""));
    }

    #[test]
    fn list_with_no_match_is_200() {
        assert!(!should_return_304("\"a1b2\"", "\"x\", \"y\""));
    }
}