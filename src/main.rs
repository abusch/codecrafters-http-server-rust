use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.context("Reading request")?;
    println!("Read {n} bytes");

    stream
        .write_all(b"HTTP/1.1 200 OK\r\n\r\n")
        .await
        .context("Writing response")?;

    Ok(())
}
