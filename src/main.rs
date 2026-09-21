mod request;
mod response;
mod router;

use std::io::{Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::thread;
use std::env;

fn main() {
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let directory = parse_directory_args();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let dir = directory.clone();
                thread::spawn(move || {
                    if let Some(req) = request::parse_request(&mut stream) {
                        let response = router::route(&req, &dir);
                        let _ = stream.write_all(&response);
                        let _ = stream.flush();
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn parse_directory_args() -> String {
    let args: Vec<String> = env::args().collect();
    args.iter().position(|a| a == "--directory")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default()
}
