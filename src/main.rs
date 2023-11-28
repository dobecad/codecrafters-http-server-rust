use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs::File,
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
                eprintln!("error: {}", e);
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
        path if path.starts_with("/echo/") => {
            echo_handler(stream, path)?;
        }
        path if path.starts_with("/user-agent") => header_handler(stream, parts)?,
        path if path.starts_with("/files/") => file_handler(stream, path, contents)?,
        _ => {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            send_response(stream, response)?;
        }
    }

    Ok(())
}

fn echo_handler(stream: TcpStream, path: &str) -> Result<()> {
    let content = path
        .split("/echo/")
        .nth(1)
        .context("missing echo content")?;
    let mut response_parts: Vec<String> = vec!["HTTP/1.1 200 Ok\r\n".to_string()];
    response_parts.push("Content-Type: text/plain\r\n".to_string());
    response_parts.push(format!("Content-Length: {}\r\n", content.len()));
    response_parts.push("\r\n".to_string());
    response_parts.push(content.to_string());
    let response = response_parts.join("");
    send_response(stream, &response)?;

    Ok(())
}

fn header_handler(stream: TcpStream, parts: Vec<String>) -> Result<()> {
    let content: Vec<String> = parts
        .iter()
        .filter(|part| part.contains(":"))
        .map(|part| part.replace(" ", ""))
        .collect();

    let mut headers: HashMap<&str, &str> = HashMap::new();
    let mut content_len = 0;
    let _ = content.iter().for_each(|part| {
        let header = part.split_once(':');
        header.map(|h| {
            if h.0 == "User-Agent" {
                headers.insert(h.0, h.1);
                content_len += h.1.len();
            }
        });
    });

    let mut response_parts: Vec<String> = vec!["HTTP/1.1 200 Ok\r\n".to_string()];
    response_parts.push("Content-Type: text/plain\r\n".to_string());
    response_parts.push(format!("Content-Length: {}\r\n", content_len));
    response_parts.push("\r\n".to_string());

    headers.values().for_each(|v| {
        response_parts.push(format!("{v}"));
    });
    let response = response_parts.join("");
    send_response(stream, &response)?;
    Ok(())
}

fn file_handler(
    stream: TcpStream,
    path: &str,
    request_contents: std::borrow::Cow<'_, str>,
) -> Result<()> {
    let mut directory_name = String::new();
    let args: Vec<_> = std::env::args().collect();
    args.get(1).map(|v| {
        if v == "--directory" {
            directory_name = args.get(2).expect("missing directory name").clone();
        }
    });

    let file_name = path.split("/files/").nth(1).context("missing file name")?;

    if request_contents.starts_with("POST") {
        upload_file(stream, &directory_name, file_name, request_contents)?;
    } else {
        read_file(stream, &directory_name, file_name)?;
    }

    Ok(())
}

fn upload_file(
    stream: TcpStream,
    directory_name: &str,
    file_name: &str,
    request_contents: std::borrow::Cow<'_, str>,
) -> Result<()> {
    let parts: Vec<String> = request_contents
        .lines()
        .map(|line| line.trim_end_matches(char::from(0)).to_string())
        .collect();
    let contents = parts[parts.len() - 1].as_bytes();
    let mut file =
        File::create(format!("{directory_name}/{file_name}")).context("failed to create file")?;
    file.write_all(&contents)
        .context("failed to write to file")?;
    send_response(stream, "HTTP/1.1 201\r\n\r\n")?;
    Ok(())
}

fn read_file(stream: TcpStream, directory_name: &str, file_name: &str) -> Result<()> {
    let file_contents = std::fs::read(format!("{directory_name}/{file_name}"));
    match file_contents {
        Ok(contents) => {
            let mut response_parts: Vec<String> = vec!["HTTP/1.1 200 Ok\r\n".to_string()];
            response_parts.push("Content-Type: application/octet-stream\r\n".to_string());
            response_parts.push(format!("Content-Length: {}\r\n", contents.len()));
            response_parts.push("\r\n".to_string());
            response_parts.push(String::from_utf8(contents).context("file not utf-8")?);
            let response = response_parts.join("");
            send_response(stream, &response)?;
        }
        Err(_) => {
            send_response(stream, "HTTP/1.1 404 Not Found\r\n\r\n")?;
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
