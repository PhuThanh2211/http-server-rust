use std::io;
use std::io::BufRead;
use crate::vhost::{Resolved, VHosts};

#[path = "../vhost.rs"]
mod vhost;

fn main() {
    let mut vh = VHosts::new();
    let mut in_table = true;

    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if in_table {
            if line.trim().is_empty() {
                in_table = false;
                continue;
            }
            let mut parts = line.split_whitespace();
            if let (Some(pat), Some(site)) = (parts.next(), parts.next()) {
                vh.add(pat, site);
            }
        } else {
            match vh.resolve(&line) {
                Resolved::Site(s) => println!("{}", s),
                Resolved::BadRequest => println!("400"),
                Resolved::NotFound => println!("404"),
            }
        }
    }
}