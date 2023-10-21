use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221")
        .await
        .context("Creating TcpListener")?;

    while let Ok((stream, _addr)) = listener.accept().await {
        println!("accepted new connection");
        handle_connection(stream).await?;
        println!("finished handling connection");
    }

    Ok(())
}

pub async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let (reader, writer) = stream.split();
    let reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    // Read the Head of the request
    let mut lines = Vec::new();
    {
        let mut lines_stream = reader.lines();
        while let Some(line) = lines_stream.next_line().await.context("Reading request")? {
            if line.trim().is_empty() {
                break;
            }
            lines.push(line);
        }
    }
    println!("Read {} lines from request", lines.len());

    let parts: Vec<&str> = lines[0].split(' ').collect();
    assert_eq!(parts.len(), 3, "Invalid request!");
    let request_path = parts[1];

    let mut response = String::new();
    if request_path == "/" {
        response.push_str("HTTP/1.1 200 OK\r\n\r\n");
    } else if let Some(content) = request_path.strip_prefix("/echo/") {
        let content_length = content.len();
        response.push_str("HTTP/1.1 200 OK\r\n");
        response.push_str("Content-Type: text/plain\r\n");
        response.push_str(&format!("Content-Length: {content_length}\r\n"));
        response.push_str("\r\n");
        response.push_str(content);
    } else {
        response.push_str("HTTP/1.1 404 Not Found\r\n");
    }

    writer
        .write_all(response.as_bytes())
        .await
        .context("Writing response")?;

    writer.flush().await.context("Flushing response")?;
    Ok(())
}
