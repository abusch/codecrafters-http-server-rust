use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use http::{Method, Request, Response};
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

pub mod http;

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

    let request = http::parse_request(reader).await?;

    let response = handle_request(request, dir)?;

    response
        .write(&mut writer)
        .await
        .context("Writing response")?;
    writer.flush().await.context("Flushing response")?;
    Ok(())
}

pub fn handle_request(request: Request, dir: &Path) -> Result<Response> {
    let response = if &request.path == "/" {
        Response::ok()
    } else if let Some(content) = request.path.strip_prefix("/echo/") {
        let content_length = content.len();
        Response::ok()
            .set_header("Content-Type", "text/plain")
            .set_header("Content-Length", content_length.to_string().as_str())
            .set_body(content.as_bytes())
    } else if request.path == "/user-agent" {
        let user_agent = request
            .headers
            .get("User-Agent")
            .context("Finding user-agent")?;
        let content_length = user_agent.len();
        Response::ok()
            .set_header("Content-Type", "text/plain")
            .set_header("Content-Length", content_length.to_string().as_str())
            .set_body(user_agent.as_bytes())
    } else if let Some(file_name) = request.path.strip_prefix("/files/") {
        let file_path = dir.join(file_name);
        match request.method {
            Method::Get => {
                let exists = file_path.try_exists().context("Checking if file exists")?;
                if exists {
                    let content = fs::read(file_path).context("Reading file content")?;
                    let content_length = content.len();

                    Response::ok()
                        .set_header("Content-Type", "application/octet-stream")
                        .set_header("Content-Length", content_length.to_string().as_str())
                        .set_body(content)
                } else {
                    Response::not_found()
                }
            }
            Method::Post => {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(file_path)
                    .context("Creating file")?;
                file.write_all(request.body.as_slice())
                    .context("Writing file")?;
                Response::created()
            }
        }
    } else {
        Response::not_found()
    };

    Ok(response)
}
