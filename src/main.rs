use anyhow::{Context, Result};
use std::{
    io::Read,
    io::Write,
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
};

fn main() -> Result<()> {
    let socketaddr = SocketAddr::from(([127, 0, 0, 1], 4221));
    let listener = TcpListener::bind(socketaddr).context("failed to bind to port")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream) {
                        println!("Error: {e}");
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 512];
    let bytes_read = stream
        .read(&mut buffer)
        .context("failed to read from stream")?;

    if bytes_read == 0 {
        return Ok(());
    }

    let contents = String::from_utf8_lossy(&buffer);
    let parts: Vec<String> = contents.lines().map(|line| line.to_string()).collect();
    let path = parts
        .iter()
        .nth(0)
        .map(|start_line| start_line.split(" ").nth(1))
        .context("missing path")?
        .context("missing starting line")?;

    match path {
        "/" => {
            let response = "HTTP/1.1 200 OK\r\n\r\n";
            send_response(stream, response)?;
        }
        _ => {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            send_response(stream, response)?;
        }
    }

    Ok(())
}

fn send_response(mut stream: TcpStream, content: &str) -> Result<()> {
    let _ = stream
        .write_all(content.as_bytes())
        .context("failed to write to TcpStream")?;
    let _ = stream.flush().context("failed to flush TcpStream")?;
    Ok(())
}
