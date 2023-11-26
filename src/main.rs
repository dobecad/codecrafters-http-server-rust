use anyhow::{Context, Result};
use std::{
    io::Read,
    io::Write,
    net::{SocketAddr, TcpListener},
};

fn main() -> Result<()> {
    let socketaddr = SocketAddr::from(([127, 0, 0, 1], 4221));
    let listener = TcpListener::bind(socketaddr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buffer: Vec<u8> = Vec::new();
                let _ = stream
                    .read(&mut buffer)
                    .context("failed to read from stream")?;
                println!("accepted new connection");
                let response = "HTTP/1.1 200 OK\r\n\r\n";
                stream
                    .write_all(response.as_bytes())
                    .context("failed to write to TcpStream")?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
