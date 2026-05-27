//! Minimal HTTP/1.1 client over a Unix domain socket.
//!
//! The OpenCoven daemon binds `~/.coven/coven.sock` and serves a plain
//! HTTP/1.1 surface (Content-Length framed, Connection: close). That's
//! the only transport the local daemon supports today, so the gateway
//! client needs a tiny client that speaks it.
//!
//! Scope is intentionally narrow: GET / POST / DELETE with JSON bodies,
//! Content-Length-framed responses, single-shot connections. Streaming,
//! WebSocket upgrades, chunked transfers, and TLS are out of scope —
//! the daemon doesn't use them. Anything more elaborate belongs behind
//! a real hyper client.

use std::path::Path;

use anyhow::{anyhow, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub struct UnixResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl UnixResponse {
    pub fn into_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        if !(200..300).contains(&self.status) {
            let preview = String::from_utf8_lossy(&self.body);
            let preview = preview.chars().take(400).collect::<String>();
            return Err(anyhow!(
                "daemon returned HTTP {} — body: {}",
                self.status,
                preview
            ));
        }
        serde_json::from_slice::<T>(&self.body).with_context(|| {
            let preview = String::from_utf8_lossy(&self.body);
            let preview = preview.chars().take(400).collect::<String>();
            format!("parse JSON response (first 400 bytes: {preview})")
        })
    }

    pub fn ensure_2xx(&self) -> Result<()> {
        if (200..300).contains(&self.status) {
            Ok(())
        } else {
            let preview = String::from_utf8_lossy(&self.body);
            let preview = preview.chars().take(400).collect::<String>();
            Err(anyhow!(
                "daemon returned HTTP {} — body: {}",
                self.status,
                preview
            ))
        }
    }
}

pub async fn request(
    socket: &Path,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    timeout: std::time::Duration,
) -> Result<UnixResponse> {
    let fut = async {
        let mut stream = UnixStream::connect(socket)
            .await
            .with_context(|| format!("connect to {}", socket.display()))?;

        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nUser-Agent: cast_agent/0.1\r\nAccept: application/json\r\nConnection: close\r\n"
        );
        if let Some(body) = body {
            head.push_str("Content-Type: application/json\r\n");
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");

        stream.write_all(head.as_bytes()).await?;
        if let Some(body) = body {
            stream.write_all(body).await?;
        }
        stream.flush().await?;

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;

        let header_end =
            find_header_end(&buf).ok_or_else(|| anyhow!("response missing header terminator"))?;
        let head = std::str::from_utf8(&buf[..header_end]).context("non-utf8 response headers")?;
        let status_line = head
            .split("\r\n")
            .next()
            .ok_or_else(|| anyhow!("empty response"))?;
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow!("malformed status line: {status_line:?}"))?;
        let body = buf[header_end + 4..].to_vec();

        Ok::<_, anyhow::Error>(UnixResponse { status, body })
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| anyhow!("unix-socket request to {path} timed out"))?
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_header_terminator() {
        let buf = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
        // Position is the start offset of \r\n\r\n (the 4-byte terminator).
        // Headers end before that offset; body starts at offset + 4.
        let pos = find_header_end(buf).expect("terminator found");
        assert_eq!(&buf[pos..pos + 4], b"\r\n\r\n");
        assert_eq!(&buf[pos + 4..], b"OK");
    }

    #[test]
    fn no_terminator_returns_none() {
        let buf = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n";
        assert_eq!(find_header_end(buf), None);
    }

    #[test]
    fn into_json_surfaces_status_for_non_2xx() {
        let resp = UnixResponse {
            status: 404,
            body: br#"{"error":"not_found"}"#.to_vec(),
        };
        let err = resp.into_json::<serde_json::Value>().unwrap_err();
        assert!(err.to_string().contains("404"));
    }
}
