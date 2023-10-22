use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = match args.next().as_deref() {
        Some("--directory") => args
            .next()
            .map(PathBuf::from)
            .context("Parsing directory argument")?,
        Some(arg) => bail!("Unknown option {arg}"),
        None => std::env::current_dir().context("Getting current directory")?,
    };
    println!("Serving files from directory: {}", dir.display());

    let dir = Arc::new(dir);

    let listener = TcpListener::bind("127.0.0.1:4221")
        .await
        .context("Creating TcpListener")?;

    while let Ok((stream, _addr)) = listener.accept().await {
        let dir = dir.clone();
        spawn(async move {
            println!("accepted new connection");
            if let Err(e) = handle_connection(stream, dir.as_ref()).await {
                println!("Error handling connection: {e}");
            } else {
                println!("finished handling connection");
            }
        });
    }

    Ok(())
}

pub async fn handle_connection(mut stream: TcpStream, dir: &Path) -> Result<()> {
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

    let parts: Vec<&str> = lines[0].split(' ').collect();
    assert_eq!(parts.len(), 3, "Invalid request!");
    let request_path = parts[1];

    if request_path == "/" {
        writer.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await?;
    } else if let Some(content) = request_path.strip_prefix("/echo/") {
        let content_length = content.len();
        writer.write_all(b"HTTP/1.1 200 OK\r\n").await?;
        writer.write_all(b"Content-Type: text/plain\r\n").await?;
        writer
            .write_all(format!("Content-Length: {content_length}\r\n").as_bytes())
            .await?;
        writer.write_all(b"\r\n").await?;
        writer.write_all(content.as_bytes()).await?;
    } else if request_path == "/user-agent" {
        let user_agent = &lines[1..]
            .iter()
            .find_map(|line| line.strip_prefix("User-Agent: "))
            .context("Finding user-agent")?;
        let content_length = user_agent.len();
        writer.write_all(b"HTTP/1.1 200 OK\r\n").await?;
        writer.write_all(b"Content-Type: text/plain\r\n").await?;
        writer
            .write_all(format!("Content-Length: {content_length}\r\n").as_bytes())
            .await?;
        writer.write_all(b"\r\n").await?;
        writer.write_all(user_agent.as_bytes()).await?;
    } else if let Some(file_name) = request_path.strip_prefix("/files/") {
        let file_path = dir.join(file_name);
        let exists = file_path.try_exists().context("Checking if file exists")?;
        if exists {
            let content = fs::read(file_path).context("Reading file content")?;
            let content_length = content.len();

            writer.write_all(b"HTTP/1.1 200 OK\r\n").await?;
            writer
                .write_all(b"Content-Type: application/octet-stream\r\n")
                .await?;
            writer
                .write_all(format!("Content-Length: {content_length}\r\n").as_bytes())
                .await?;
            writer.write_all(b"\r\n").await?;
            writer.write_all(&content).await?;
        } else {
            writer.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await?;
        }
    } else {
        writer.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await?;
    }

    writer.flush().await.context("Flushing response")?;
    Ok(())
}
