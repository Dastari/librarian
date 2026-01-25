//! NNTP Protocol Client
//!
//! Native Rust implementation of the NNTP (Network News Transfer Protocol) client
//! for downloading articles from Usenet servers.
//!
//! # NNTP Commands Used
//!
//! - `AUTHINFO USER/PASS` - Authentication
//! - `GROUP` - Select a newsgroup
//! - `ARTICLE` - Retrieve full article (headers + body)
//! - `BODY` - Retrieve article body only
//! - `STAT` - Check if article exists
//! - `QUIT` - Close connection
//!
//! # Connection Flow
//!
//! 1. Connect to server (plain or TLS)
//! 2. Receive greeting (200/201)
//! 3. Authenticate if required
//! 4. Select group and retrieve articles
//! 5. Close connection

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{Result, anyhow};
use native_tls::TlsConnector;
use tracing::{debug, info};

/// NNTP client configuration
#[derive(Debug, Clone)]
pub struct NntpConfig {
    /// Server hostname
    pub host: String,
    /// Server port (usually 563 for TLS, 119 for plain)
    pub port: u16,
    /// Use TLS/SSL
    pub use_tls: bool,
    /// Username (optional)
    pub username: Option<String>,
    /// Password (optional)
    pub password: Option<String>,
    /// Connection timeout
    pub timeout: Duration,
}

impl Default for NntpConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 563,
            use_tls: true,
            username: None,
            password: None,
            timeout: Duration::from_secs(30),
        }
    }
}

/// NNTP response
#[derive(Debug)]
pub struct NntpResponse {
    /// Response code (e.g., 200, 211, 220, etc.)
    pub code: u16,
    /// Response message
    pub message: String,
    /// Multi-line data (for ARTICLE, BODY, etc.)
    pub data: Option<Vec<u8>>,
}

impl NntpResponse {
    /// Check if response indicates success (2xx codes)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.code)
    }

    /// Check if authentication is required (480)
    pub fn requires_auth(&self) -> bool {
        self.code == 480
    }
}

/// NNTP client for a single connection
pub struct NntpClient {
    config: NntpConfig,
    // The connection is stored as an enum to handle both TLS and plain
    connection: Option<NntpConnection>,
}

enum NntpConnection {
    Plain(BufReader<TcpStream>),
    Tls(BufReader<native_tls::TlsStream<TcpStream>>),
}

impl NntpClient {
    /// Create a new NNTP client (not connected)
    pub fn new(config: NntpConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    /// Connect to the NNTP server
    pub fn connect(&mut self) -> Result<NntpResponse> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        debug!(addr = %addr, use_tls = self.config.use_tls, "Connecting to NNTP server");

        let stream = TcpStream::connect_timeout(
            &addr.parse()?,
            self.config.timeout,
        )?;
        stream.set_read_timeout(Some(self.config.timeout))?;
        stream.set_write_timeout(Some(self.config.timeout))?;

        if self.config.use_tls {
            let connector = TlsConnector::new()?;
            let tls_stream = connector.connect(&self.config.host, stream)?;
            self.connection = Some(NntpConnection::Tls(BufReader::new(tls_stream)));
        } else {
            self.connection = Some(NntpConnection::Plain(BufReader::new(stream)));
        }

        // Read greeting
        let response = self.read_response()?;

        info!(
            host = %self.config.host,
            code = response.code,
            "Connected to NNTP server"
        );

        // Authenticate if credentials provided
        let needs_auth = response.requires_auth() || response.code == 200 || response.code == 201;
        if needs_auth {
            if let (Some(user), Some(pass)) = (self.config.username.clone(), self.config.password.clone()) {
                self.authenticate(&user, &pass)?;
            }
        }

        Ok(response)
    }

    /// Authenticate with the server
    pub fn authenticate(&mut self, username: &str, password: &str) -> Result<NntpResponse> {
        debug!(username = %username, "Authenticating");

        // Send username
        self.send_command(&format!("AUTHINFO USER {}", username))?;
        let response = self.read_response()?;

        if response.code != 381 {
            if response.is_success() {
                return Ok(response);
            }
            return Err(anyhow!("Authentication failed at USER: {} {}", response.code, response.message));
        }

        // Send password
        self.send_command(&format!("AUTHINFO PASS {}", password))?;
        let response = self.read_response()?;

        if !response.is_success() {
            return Err(anyhow!("Authentication failed: {} {}", response.code, response.message));
        }

        info!("Authentication successful");
        Ok(response)
    }

    /// Select a newsgroup
    pub fn group(&mut self, group_name: &str) -> Result<NntpResponse> {
        self.send_command(&format!("GROUP {}", group_name))?;
        self.read_response()
    }

    /// Check if an article exists
    pub fn stat(&mut self, message_id: &str) -> Result<bool> {
        let mid = normalize_message_id(message_id);
        self.send_command(&format!("STAT {}", mid))?;
        let response = self.read_response()?;
        Ok(response.code == 223)
    }

    /// Retrieve article body by message ID
    pub fn body(&mut self, message_id: &str) -> Result<Vec<u8>> {
        let mid = normalize_message_id(message_id);
        self.send_command(&format!("BODY {}", mid))?;
        let response = self.read_multiline_response()?;

        if !response.is_success() {
            return Err(anyhow!("BODY failed: {} {}", response.code, response.message));
        }

        response.data.ok_or_else(|| anyhow!("No body data received"))
    }

    /// Retrieve full article (headers + body)
    pub fn article(&mut self, message_id: &str) -> Result<Vec<u8>> {
        let mid = normalize_message_id(message_id);
        self.send_command(&format!("ARTICLE {}", mid))?;
        let response = self.read_multiline_response()?;

        if !response.is_success() {
            return Err(anyhow!("ARTICLE failed: {} {}", response.code, response.message));
        }

        response.data.ok_or_else(|| anyhow!("No article data received"))
    }

    /// Quit and close the connection
    pub fn quit(&mut self) -> Result<()> {
        if self.connection.is_some() {
            let _ = self.send_command("QUIT");
            let _ = self.read_response();
            self.connection = None;
        }
        Ok(())
    }

    /// Send a command to the server
    fn send_command(&mut self, command: &str) -> Result<()> {
        let conn = self.connection.as_mut().ok_or_else(|| anyhow!("Not connected"))?;

        let cmd = format!("{}\r\n", command);
        
        // Don't log passwords
        let log_cmd = if command.starts_with("AUTHINFO PASS") {
            "AUTHINFO PASS ****"
        } else {
            command
        };
        debug!(command = %log_cmd, "Sending NNTP command");

        match conn {
            NntpConnection::Plain(reader) => {
                reader.get_mut().write_all(cmd.as_bytes())?;
            }
            NntpConnection::Tls(reader) => {
                reader.get_mut().write_all(cmd.as_bytes())?;
            }
        }

        Ok(())
    }

    /// Read a single-line response
    fn read_response(&mut self) -> Result<NntpResponse> {
        let conn = self.connection.as_mut().ok_or_else(|| anyhow!("Not connected"))?;

        let mut line = String::new();
        
        match conn {
            NntpConnection::Plain(reader) => {
                reader.read_line(&mut line)?;
            }
            NntpConnection::Tls(reader) => {
                reader.read_line(&mut line)?;
            }
        }

        let line = line.trim_end();
        debug!(response = %line, "Received NNTP response");

        parse_response_line(line)
    }

    /// Read a multi-line response (terminated by ".")
    fn read_multiline_response(&mut self) -> Result<NntpResponse> {
        let conn = self.connection.as_mut().ok_or_else(|| anyhow!("Not connected"))?;

        // Read first line (status)
        let mut first_line = String::new();
        match conn {
            NntpConnection::Plain(reader) => reader.read_line(&mut first_line)?,
            NntpConnection::Tls(reader) => reader.read_line(&mut first_line)?,
        };

        let mut response = parse_response_line(first_line.trim_end())?;

        if !response.is_success() {
            return Ok(response);
        }

        // Read data lines until "."
        let mut data = Vec::new();
        loop {
            let mut line = String::new();
            match conn {
                NntpConnection::Plain(reader) => reader.read_line(&mut line)?,
                NntpConnection::Tls(reader) => reader.read_line(&mut line)?,
            };

            // Remove CRLF
            let line = line.trim_end_matches(|c| c == '\r' || c == '\n');

            if line == "." {
                break;
            }

            // Handle dot-stuffing (lines starting with ".." should be ".")
            let content = if line.starts_with("..") {
                &line[1..]
            } else {
                line
            };

            data.extend_from_slice(content.as_bytes());
            data.push(b'\n');
        }

        response.data = Some(data);
        Ok(response)
    }
}

impl Drop for NntpClient {
    fn drop(&mut self) {
        if self.connection.is_some() {
            let _ = self.quit();
        }
    }
}

/// Normalize a message ID (ensure angle brackets)
fn normalize_message_id(id: &str) -> String {
    if id.starts_with('<') && id.ends_with('>') {
        id.to_string()
    } else {
        format!("<{}>", id)
    }
}

/// Parse a response line into code and message
fn parse_response_line(line: &str) -> Result<NntpResponse> {
    if line.len() < 3 {
        return Err(anyhow!("Invalid NNTP response: {}", line));
    }

    let code: u16 = line[..3].parse()
        .map_err(|_| anyhow!("Invalid response code: {}", line))?;

    let message = if line.len() > 4 {
        line[4..].to_string()
    } else {
        String::new()
    };

    Ok(NntpResponse {
        code,
        message,
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_message_id() {
        assert_eq!(
            normalize_message_id("abc@example.com"),
            "<abc@example.com>"
        );
        assert_eq!(
            normalize_message_id("<abc@example.com>"),
            "<abc@example.com>"
        );
    }

    #[test]
    fn test_parse_response_line() {
        let resp = parse_response_line("200 Hello, you are welcome!").unwrap();
        assert_eq!(resp.code, 200);
        assert_eq!(resp.message, "Hello, you are welcome!");
        assert!(resp.is_success());

        let resp = parse_response_line("480 Authentication required").unwrap();
        assert_eq!(resp.code, 480);
        assert!(resp.requires_auth());
    }
}
