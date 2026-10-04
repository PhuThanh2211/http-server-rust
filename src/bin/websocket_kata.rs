#[path = "../websocket.rs"]
mod websocket;

use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        // splitn(5) so an empty key field is kept as ""
        let f: Vec<&str> = line.splitn(5, '|').collect();
        if f.len() < 5 {
            println!("400");
            continue;
        }
        match websocket::validate_handshake(f[0], f[1], f[2], f[3], f[4]) {
            Some(accept) => println!("101 {}", accept),
            None => println!("400"),
        }
    }
}