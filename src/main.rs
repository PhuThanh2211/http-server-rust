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

                let response = match path {
                    "/" => "HTTP/1.1 200 OK\r\n\r\n",
                    _ => "HTTP/1.1 404 Not Found\r\n\r\n",
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
