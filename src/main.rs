mod request;
mod response;
mod router;
mod logging;
mod static_files;
#[cfg(test)]
mod test_utils;

use std::io::{ErrorKind, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::{panic, thread};
use std::env;
use std::net::Shutdown;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn main() {
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    // Non-blocking accept loop: periodically check the shutdown flag
    // even when no new connection has arrived yet.
    listener.set_nonblocking(true).unwrap();

    let directory = parse_directory_args();

    // Shared flag: false = running normally, true = shutting down.
    let shutting_down = Arc::new(AtomicBool::new(false));
    {
        let flag = shutting_down.clone();
        ctrlc::set_handler(move || {
            println!("\nShutdown signal received. Finishing in-flight requests...");
            flag.store(true, Ordering::SeqCst);
        }).expect("Error setting Ctrl+C handler");
    }

    // Track spawned threads so we can wait for them to finish before exiting.
    let mut handles = Vec::new();

    for stream in listener.incoming() {
        if shutting_down.load(Ordering::SeqCst) {
            println!("Shutting down: no longer accepting new connections.");
            break;
        }

        match stream {
            Ok(mut stream) => {
                // set_nonblocking on the listener makes accept() non-blocking;
                // reset the per-connection stream back to blocking so read()/write()
                // behave normally for the rest of the request handling.
                let _ = stream.set_nonblocking(false);

                // If no data arrives within 30s, .read() returns an error
                // instead of blocking the thread forever.
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));

                let dir = directory.clone();
                let peer_ip = stream.peer_addr()
                    .map(|a| a.ip().to_string())
                    .unwrap_or_else(|_| "-".to_string());

                let handle = thread::spawn(move || {
                    loop {
                        let start = Instant::now();

                        let req = match request::parse_request(&mut stream) {
                            Some(r) => r,
                            None => break, // Connection closed or no data
                        };

                        let should_close = req.header("connection")
                            .map(|v| v.eq_ignore_ascii_case("close"))
                            .unwrap_or(false);

                        let mut response = match panic::catch_unwind(AssertUnwindSafe(|| {
                            router::route(&req, &dir)
                        })) {
                            Ok(resp) => resp,
                            Err(_) => {
                                eprintln!("handler panicked while processing {} {}", req.method, req.path); // server-side log only
                                response::internal_server_error("Internal Server Error\n")
                            }
                        };

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

                handles.push(handle);
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // No incoming connection yet (expected, since listener is non-blocking).
                // Avoid busy-spinning the CPU while polling.
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    println!("Waiting for {} in-flight request(s) to finish...", handles.len());
    for handle in handles {
        let _ = handle.join();
    }
    println!("Shutdown complete.");
}

fn parse_directory_args() -> String {
    let args: Vec<String> = env::args().collect();
    args.iter().position(|a| a == "--directory")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default()
}
