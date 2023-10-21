use anyhow::{Context, Result};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221")
        .await
        .context("Creating TcpListener")?;

    while let Ok((stream, addr)) = listener.accept().await {
        println!("accepted new connection");
    }

    Ok(())
}
