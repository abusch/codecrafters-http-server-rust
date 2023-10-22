use std::{collections::HashMap, fmt::Display, str::FromStr};

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, thiserror::Error)]
#[error("Parse error: {0}")]
pub struct ParseError(String);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl FromStr for Method {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            _ => Err(ParseError("Invalid HTTP Method {s}".to_string())),
        }
    }
}

pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    #[default]
    Ok,
    Created,
    NotFound,
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StatusCode::Ok => "200 OK",
            StatusCode::Created => "201 Created",
            StatusCode::NotFound => "404 Not Found",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn ok() -> Self {
        Self {
            status: StatusCode::Ok,
            ..Default::default()
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: StatusCode::NotFound,
            ..Default::default()
        }
    }

    pub fn created() -> Self {
        Self {
            status: StatusCode::Created,
            ..Default::default()
        }
    }

    pub fn set_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_owned(), value.to_owned());
        self
    }

    pub fn set_body(mut self, body: impl AsRef<[u8]>) -> Self {
        self.body.extend_from_slice(body.as_ref());
        self
    }

    pub async fn write(self, mut writer: impl AsyncWrite + Unpin) -> Result<()> {
        // status line
        let status_line = format!("HTTP/1.1 {}\r\n", self.status);
        writer.write_all(status_line.as_bytes()).await?;

        // headers
        for (k, v) in self.headers.into_iter() {
            writer.write_all(format!("{k}: {v}\r\n").as_bytes()).await?;
        }

        // blank line
        writer.write_all(b"\r\n").await?;

        // body
        writer.write_all(&self.body).await?;

        Ok(())
    }
}

pub async fn parse_request(reader: impl AsyncBufRead + Unpin) -> Result<Request> {
    // Read the Head of the request
    let mut lines = Vec::new();
    let mut lines_stream = reader.lines();
    while let Some(line) = lines_stream.next_line().await.context("Reading request")? {
        if line.trim().is_empty() {
            break;
        }
        lines.push(line);
    }

    let parts: Vec<&str> = lines[0].split(' ').collect();
    if parts.len() != 3 {
        bail!("Invalid HTTP request");
    }
    let method = parts[0].parse()?;
    let path = parts[1].to_owned();

    let headers: HashMap<String, String> = lines[1..]
        .iter()
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
        })
        .collect();

    let body = if let Some(length) = headers.get("Content-Length") {
        let length = length.parse::<usize>().context("Invalid content length")?;
        // Read body
        let mut buf = vec![0u8; length];
        lines_stream
            .get_mut()
            .read_exact(&mut buf)
            .await
            .context("Reading body")?;
        buf
    } else {
        vec![]
    };

    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}
