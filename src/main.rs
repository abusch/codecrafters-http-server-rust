use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufStream};
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

pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut stream = BufStream::new(stream);

    // Read first line
    let mut line = String::new();
    stream.read_line(&mut line).await.context("Read request")?;

    let parts: Vec<&str> = line.split(' ').collect();
    assert_eq!(parts.len(), 3, "Invalid request!");
    let request_path = parts[1];

    let response = match request_path {
        "/" => "HTTP/1.1 200 OK\r\n\r\n",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n",
    };

    stream
        .write_all(response.as_bytes())
        .await
        .context("Writing response")?;

    stream.flush().await.context("Flushing response")?;
    Ok(())
}
