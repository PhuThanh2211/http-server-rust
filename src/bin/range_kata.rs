use std::io::{self, BufRead};
use crate::range::{resolve_range, RangeOutcome};

#[path = "../range.rs"]
mod range;

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }

        let (size_s, spec) = line.split_once('|').unwrap_or((line.as_str(), ""));
        let size: u64 = size_s.trim().parse().unwrap_or(0);

        match resolve_range(size, spec) {
            RangeOutcome::Slice { start, end} => {
                println!("206 {}-{}/{} {}", start, end, size, end - start + 1)
            }
            RangeOutcome::Unsatisfiable => println!("416 */{}", size),
        }
    }
}