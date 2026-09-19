use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::thread;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    let args: Vec<String> = env::args().collect();
    let mut directory = String::new();

    for i in 0..args.len() {
        if args[i] == "--directory" && i + 1 < args.len() {
            directory = args[i + 1].clone();
            break;
        }
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir = directory.clone();
                thread::spawn(move || {
                    let mut stream = stream;
                    let mut buf = [0u8; 1024];
                    let bytes_read = match stream.read(&mut buf) {
                        Ok(n) if n > 0 => n,
                        _ => return
                    };
                    let request = String::from_utf8_lossy(&buf[..bytes_read]);

                    let first_line = request.lines().next().unwrap_or("");
                    let path = first_line.split_whitespace().nth(1).unwrap_or("");
                    let method = first_line.split_whitespace().nth(0).unwrap_or("");

                    let mut user_agent = "";
                    for line in request.lines() {
                        if let Some(_val) = line.to_ascii_lowercase().strip_prefix("user-agent:") {
                            user_agent = line["user-agent:".len()..].trim();
                            break;
                        }
                    }

                    let mut content_length: usize = 0;
                    for line in request.lines() {
                        if let Some(val) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            content_length = val.trim().parse().unwrap_or(0);
                            break;
                        }
                    }

                    // Find where headers end in raw bytes
                    let header_end = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(bytes_read);
                    let mut body = buf[header_end..bytes_read].to_vec();

                    while body.len() < content_length {
                        let mut chunk = [0u8; 1024];
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => body.extend_from_slice(&chunk[..n]),
                            Err(_) => break,
                        }
                    }

                    let response = if method == "POST" {
                        if let Some(filename) = path.strip_prefix("/files/") {
                            let file_path = Path::new(&dir).join(filename);
                            match fs::write(file_path, &body) {
                                Ok(_) => "HTTP/1.1 201 Created\r\n\r\n".to_string(),
                                Err(_) => "HTTP/1.1 500 Internal Server Error\r\n\r\n".to_string(),
                            }
                        } else {
                            "HTTP/1.1 404 Not Found\r\n\r\n".to_string()
                        }
                    } else if path == "/" {
                        "HTTP/1.1 200 OK\r\n\r\n".to_string()
                    } else if let Some(echo_str) = path.strip_prefix("/echo/") {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            echo_str.len(),
                            echo_str
                        )
                    } else if let Some(filename) = path.strip_prefix("/files/") {
                        let file_path = Path::new(&dir).join(filename);

                        if let Ok(contents) = fs::read(file_path) {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                contents.len(),
                                String::from_utf8_lossy(&contents)
                            )
                        } else {
                            "HTTP/1.1 404 Not Found\r\n\r\n".to_string()
                        }

                    } else if path == "/user-agent" {
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            user_agent.len(),
                            user_agent
                        )
                    } else {
                        "HTTP/1.1 404 Not Found\r\n\r\n".to_string()
                    };

                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
