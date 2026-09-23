mod request;
mod response;
mod router;
mod logging;
mod static_files;

use std::io::{Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::thread;
use std::env;
use std::net::Shutdown;
use std::time::Instant;

fn main() {
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let directory = parse_directory_args();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let dir = directory.clone();
                let peer_ip = stream.peer_addr()
                    .map(|a| a.ip().to_string())
                    .unwrap_or_else(|_| "-".to_string());

                thread::spawn(move || {
                    loop {
                        let start = Instant::now();

                        let req = match request::parse_request(&mut stream) {
                            Some(r) => r,
                            None => break, // Connection closed or no data
                        };

                        let should_close = req.header("connection")
                            .map(|v| v.eq_ignore_ascii_case("close"))
                            .unwrap_or(false);

                        let mut response = router::route(&req, &dir);
                        if should_close {
                            response = response::add_header(response, "Connection: close");
                        }

                        // --- Access log line, right before writing the response ---
                        let status = logging::extract_status(&response);
                        let bytes = logging::extract_body_length(&response);
                        let latency_ms = start.elapsed().as_millis();
                        let user_agent = req.header("user-agent").unwrap_or("-");

                        logging::log_request(&logging::LogEntry {
                            ip: peer_ip.clone(),
                            method: &req.method,
                            path: &req.path,
                            version: &req.version,
                            status,
                            bytes,
                            user_agent,
                            latency_ms
                        });

                        if stream.write_all(&response).is_err() {
                            break;
                        }

                        let _ = stream.flush();

                        if should_close {
                            // explicitly closes both read/write halves of the TCP connection immediately
                            // after sending the response
                            let _ = stream.shutdown(Shutdown::Both);
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
