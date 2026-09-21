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
                    loop {
                        let req = match request::parse_request(&mut stream) {
                            Some(r) => r,
                            None => break, // Connection closed or no data
                        };

                        let should_close = req.header("connection")
                            .map(|v| v.eq_ignore_ascii_case("close"))
                            .unwrap_or(false);

                        let response = router::route(&req, &dir);
                        if stream.write_all(&response).is_err() {
                            break;
                        }

                        let _ = stream.flush();

                        if should_close {
                            break;
                        }
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
