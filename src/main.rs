use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf = [0u8; 1024];
                let bytes_read = stream.read(&mut buf).unwrap();
                let request = String::from_utf8_lossy(&buf[..bytes_read]);

                let first_line = request.lines().next().unwrap_or("");
                let path = first_line.split_whitespace().nth(1).unwrap_or("");


                let mut user_agent = "";
                for line in request.lines() {
                    if let Some(_val) = line.to_ascii_lowercase().strip_prefix("user-agent:") {
                        user_agent = line["user-agent:".len()..].trim();
                        break;
                    }
                }

                let response = if path == "/" {
                    "HTTP/1.1 200 OK\r\n\r\n".to_string()
                } else if let Some(echo_str) = path.strip_prefix("/echo/") {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        echo_str.len(),
                        echo_str
                    )
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
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
