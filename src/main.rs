use std::io::Write;
#[allow(unused_imports)]
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Redis Server listening here with port {}!!!", 4221);

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
                stream.flush();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
