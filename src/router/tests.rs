use super::*;
use crate::test_utils::request;

fn status_line(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).lines().next().unwrap_or("").to_string()
}

#[test]
fn exact_route_returns_200() {
    assert!(status_line(route(&request("GET", "/about"), ""))
        .starts_with("HTTP/1.1 200"));
}

#[test]
fn wrong_method_returns_405_with_allow() {
    let resp = String::from_utf8_lossy(&route(&request("DELETE", "/about"), "")).to_string();
    assert!(resp.starts_with("HTTP/1.1 405"));
    assert!(resp.contains("Allow: GET"));
}

#[test]
fn unknown_path_returns_404() {
    assert!(status_line(route(&request("GET", "/nope"), "")).starts_with("HTTP/1.1 404"));
}

#[test]
fn pattern_route_binds_params() {
    let resp = String::from_utf8_lossy(&route(&request("GET", "/users/42"), "")).to_string();
    assert!(resp.contains("get_user id=42"));
}
