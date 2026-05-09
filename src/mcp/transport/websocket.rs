//! WebSocket transport for MCP
//!
//! This module provides WebSocket based transport for communicating with remote MCP servers.

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, TransportConfig};
use anyhow::{Context, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, client_async_with_config, connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message},
};
use tracing::debug;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSplitWrite = futures_util::stream::SplitSink<WsStream, Message>;
type WsSplitRead = futures_util::stream::SplitStream<WsStream>;

/// WebSocket transport for MCP
///
/// Supports lazy connection - the WebSocket connection is established
/// on the first request if not already connected.
pub struct WebSocketMcpTransport {
    url: String,
    headers: Vec<(String, String)>,
    /// Optional proxy URL (e.g., "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080").
    proxy_url: Option<String>,
    write: Mutex<Option<WsSplitWrite>>,
    read: Mutex<Option<WsSplitRead>>,
    connected: Mutex<bool>,
}

impl WebSocketMcpTransport {
    /// Create a new WebSocket MCP transport without connecting yet.
    ///
    /// The actual connection will be established lazily on the first request.
    /// This is useful because `McpClient::new()` is not async.
    pub fn new_lazy(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::WebSocket { url, headers } => {
                let mut http_headers = Vec::new();
                if let Some(headers_map) = headers {
                    for (key, value) in headers_map {
                        http_headers.push((key.clone(), value.clone()));
                    }
                }

                debug!(
                    "Created lazy WebSocket transport (will connect on first request): {}",
                    url
                );

                Ok(Self {
                    url: url.clone(),
                    headers: http_headers,
                    proxy_url: None,
                    write: Mutex::new(None),
                    read: Mutex::new(None),
                    connected: Mutex::new(false),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid transport configuration for WebSocket transport"
            )),
        }
    }

    /// Set the proxy URL for this transport.
    pub fn with_proxy(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    /// Ensure the WebSocket connection is established.
    ///
    /// If already connected, this is a no-op.
    /// If not connected, this will establish the connection.
    async fn ensure_connected(&self) -> Result<()> {
        let mut connected = self.connected.lock().await;
        if *connected {
            return Ok(());
        }

        debug!("Establishing WebSocket connection to: {}", self.url);

        // Build request with optional headers
        let mut request = self.url.as_str().into_client_request()?;

        if !self.headers.is_empty() {
            let headers = request.headers_mut();
            for (key, value) in &self.headers {
                let header_name =
                    tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(key.as_bytes())
                        .context(format!("Invalid header name: {}", key))?;
                let header_value =
                    tokio_tungstenite::tungstenite::http::HeaderValue::from_str(value)
                        .context(format!("Invalid header value for {}: {}", key, value))?;
                headers.insert(header_name, header_value);
            }
        }

        let ws_stream = if let Some(ref proxy_url) = self.proxy_url {
            // Connect through proxy
            self.connect_via_proxy(request, proxy_url).await?
        } else {
            // Direct connection
            let (ws, _) = connect_async(request)
                .await
                .context("Failed to connect to WebSocket server")?;
            ws
        };

        debug!("WebSocket connection established");

        let (write, read) = ws_stream.split();

        *self.write.lock().await = Some(write);
        *self.read.lock().await = Some(read);
        *connected = true;

        Ok(())
    }

    /// Connect WebSocket through a proxy.
    async fn connect_via_proxy(
        &self,
        request: tokio_tungstenite::tungstenite::http::Request<()>,
        proxy_url: &str,
    ) -> Result<WsStream> {
        let host = request
            .uri()
            .host()
            .ok_or_else(|| anyhow::anyhow!("No host in WebSocket URL"))?
            .to_string();
        let port = request
            .uri()
            .port_u16()
            .unwrap_or(if self.url.starts_with("wss://") {
                443
            } else {
                80
            });
        let is_tls = self.url.starts_with("wss://");

        tracing::info!(proxy = %proxy_url, host = %host, "Connecting MCP WebSocket via proxy");

        if proxy_url.starts_with("socks5://") || proxy_url.starts_with("socks5h://") {
            self.connect_via_socks5_proxy(request, &host, port, is_tls, proxy_url)
                .await
        } else {
            self.connect_via_http_proxy(request, &host, port, is_tls, proxy_url)
                .await
        }
    }

    /// Connect WebSocket through an HTTP CONNECT proxy.
    async fn connect_via_http_proxy(
        &self,
        request: tokio_tungstenite::tungstenite::http::Request<()>,
        host: &str,
        port: u16,
        is_tls: bool,
        proxy_url: &str,
    ) -> Result<WsStream> {
        let proxy_uri: url::Url = proxy_url.parse()?;
        let proxy_host = proxy_uri
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in proxy URL"))?
            .to_string();
        let proxy_port = proxy_uri
            .port_or_known_default()
            .ok_or_else(|| anyhow::anyhow!("No port in proxy URL"))?;

        let mut stream = TcpStream::connect((&*proxy_host, proxy_port)).await?;

        let mut connect_req = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            host, port, host, port
        );

        let username = proxy_uri.username();
        if !username.is_empty() {
            let password = proxy_uri.password().unwrap_or("");
            let credentials = base64::engine::general_purpose::STANDARD
                .encode(format!("{}:{}", username, password));
            connect_req.push_str(&format!("Proxy-Authorization: basic {}\r\n", credentials));
        }
        connect_req.push_str("\r\n");

        stream.write_all(connect_req.as_bytes()).await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        if !response_line.contains("200") {
            anyhow::bail!(
                "HTTP CONNECT proxy returned non-200: {}",
                response_line.trim()
            );
        }

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line == "\r\n" || line.is_empty() {
                break;
            }
        }

        let stream = reader.into_inner();

        if is_tls {
            let tls_connector = native_tls::TlsConnector::new()?;
            let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
                .connect(host, stream)
                .await?;
            let ws_stream = MaybeTlsStream::NativeTls(tls_stream);
            let (ws, _) = client_async_with_config(request, ws_stream, None).await?;
            Ok(ws)
        } else {
            let ws_stream = MaybeTlsStream::Plain(stream);
            let (ws, _) = client_async_with_config(request, ws_stream, None).await?;
            Ok(ws)
        }
    }

    /// Connect WebSocket through a SOCKS5 proxy.
    async fn connect_via_socks5_proxy(
        &self,
        request: tokio_tungstenite::tungstenite::http::Request<()>,
        host: &str,
        port: u16,
        is_tls: bool,
        proxy_url: &str,
    ) -> Result<WsStream> {
        let proxy_uri: url::Url = proxy_url.parse()?;
        let proxy_host = proxy_uri
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("No host in proxy URL"))?
            .to_string();
        let proxy_port = proxy_uri
            .port_or_known_default()
            .ok_or_else(|| anyhow::anyhow!("No port in proxy URL"))?;

        let proxy_stream = TcpStream::connect((&*proxy_host, proxy_port)).await?;

        let (username, password) = if proxy_uri.username().is_empty() {
            (String::new(), String::new())
        } else {
            (
                proxy_uri.username().to_string(),
                proxy_uri.password().unwrap_or("").to_string(),
            )
        };

        let stream = socks5_handshake(proxy_stream, host, port, &username, &password).await?;

        if is_tls {
            let tls_connector = native_tls::TlsConnector::new()?;
            let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
                .connect(host, stream)
                .await?;
            let ws_stream = MaybeTlsStream::NativeTls(tls_stream);
            let (ws, _) = client_async_with_config(request, ws_stream, None).await?;
            Ok(ws)
        } else {
            let ws_stream = MaybeTlsStream::Plain(stream);
            let (ws, _) = client_async_with_config(request, ws_stream, None).await?;
            Ok(ws)
        }
    }

    /// Mark connection as disconnected
    async fn mark_disconnected(&self) {
        *self.connected.lock().await = false;
    }
}

#[async_trait::async_trait]
impl McpTransport for WebSocketMcpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Ensure we are connected
        self.ensure_connected().await?;

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC request")?;

        debug!("Sending WebSocket request: {}", request_json);

        // Send request
        {
            let mut write_guard = self.write.lock().await;
            if write_guard.is_none() {
                return Err(anyhow::anyhow!("WebSocket connection is closed"));
            }

            let write_stream = write_guard.as_mut().unwrap();
            write_stream
                .send(Message::Text(request_json))
                .await
                .context("Failed to send WebSocket message")?;
        }

        // Receive response
        let mut read_guard = self.read.lock().await;
        if read_guard.is_none() {
            return Err(anyhow::anyhow!("WebSocket connection is closed"));
        }

        let read_stream = read_guard.as_mut().unwrap();

        let message = read_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No response from WebSocket server"))?
            .context("Failed to receive WebSocket message")?;

        let response_text = match message {
            Message::Text(text) => text,
            Message::Close(_) => {
                self.mark_disconnected().await;
                return Err(anyhow::anyhow!("WebSocket connection closed by server"));
            }
            Message::Ping(data) => {
                // Respond to ping with pong
                let mut write_guard = self.write.lock().await;
                if let Some(write_stream) = write_guard.as_mut() {
                    let _ = write_stream.send(Message::Pong(data)).await;
                }
                drop(write_guard);
                drop(read_guard);
                // Try to read the next message by recursively calling
                // Note: we need to rebuild the request since we consumed it
                return Err(anyhow::anyhow!(
                    "Received ping during request, please retry"
                ));
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unexpected WebSocket message type: {:?}",
                    message
                ));
            }
        };

        debug!("Received WebSocket response: {}", response_text);

        let json_response: JsonRpcResponse =
            serde_json::from_str(&response_text).context("Failed to parse JSON-RPC response")?;

        Ok(json_response)
    }

    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        // Ensure we are connected
        self.ensure_connected().await?;

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC notification")?;

        debug!("Sending WebSocket notification: {}", request_json);

        let mut write_guard = self.write.lock().await;
        if write_guard.is_none() {
            return Err(anyhow::anyhow!("WebSocket connection is closed"));
        }

        let write_stream = write_guard.as_mut().unwrap();

        write_stream
            .send(Message::Text(request_json))
            .await
            .context("Failed to send WebSocket notification")?;

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down WebSocket transport");

        // Close write stream
        let mut write_guard = self.write.lock().await;
        if let Some(mut write_stream) = write_guard.take() {
            let _ = write_stream.close().await;
        }
        drop(write_guard);

        // Clear read stream
        let mut read_guard = self.read.lock().await;
        *read_guard = None;
        drop(read_guard);

        *self.connected.lock().await = false;

        debug!("WebSocket transport shut down");

        Ok(())
    }
}

/// Perform a SOCKS5 handshake on a TCP stream.
async fn socks5_handshake(
    mut stream: TcpStream,
    target_host: &str,
    target_port: u16,
    username: &str,
    password: &str,
) -> Result<TcpStream> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Step 1: Client greeting
    if username.is_empty() {
        stream.write_all(&[0x05, 0x01, 0x00]).await?;
    } else {
        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await?;
    }
    stream.flush().await?;

    // Server choice
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;
    if buf[0] != 0x05 {
        anyhow::bail!("SOCKS5 proxy returned invalid version: {}", buf[0]);
    }

    // Step 2: Authenticate if required
    if buf[1] == 0x02 {
        let ulen = username.len() as u8;
        let plen = password.len() as u8;
        let mut auth_req = Vec::with_capacity(3 + username.len() + password.len());
        auth_req.push(0x01);
        auth_req.push(ulen);
        auth_req.extend_from_slice(username.as_bytes());
        auth_req.push(plen);
        auth_req.extend_from_slice(password.as_bytes());
        stream.write_all(&auth_req).await?;
        stream.flush().await?;

        let mut auth_resp = [0u8; 2];
        stream.read_exact(&mut auth_resp).await?;
        if auth_resp[1] != 0x00 {
            anyhow::bail!("SOCKS5 proxy authentication failed");
        }
    } else if buf[1] != 0x00 {
        anyhow::bail!("SOCKS5 proxy returned unsupported auth method: {}", buf[1]);
    }

    // Step 3: Connect request
    let host_bytes = target_host.as_bytes();
    let mut connect_req = Vec::with_capacity(6 + host_bytes.len());
    connect_req.push(0x05); // VER
    connect_req.push(0x01); // CMD: CONNECT
    connect_req.push(0x00); // RSV
    connect_req.push(0x03); // ATYP: DOMAINNAME
    connect_req.push(host_bytes.len() as u8);
    connect_req.extend_from_slice(host_bytes);
    connect_req.extend_from_slice(&target_port.to_be_bytes());
    stream.write_all(&connect_req).await?;
    stream.flush().await?;

    // Read connect response
    let mut resp = [0u8; 4];
    stream.read_exact(&mut resp).await?;
    if resp[1] != 0x00 {
        anyhow::bail!("SOCKS5 proxy connect failed with code: {}", resp[1]);
    }

    // Read remaining address based on type
    match resp[3] {
        0x01 => {
            let mut addr = [0u8; 4 + 2];
            stream.read_exact(&mut addr).await?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut addr = vec![0u8; len_buf[0] as usize + 2];
            stream.read_exact(&mut addr).await?;
        }
        0x04 => {
            let mut addr = [0u8; 16 + 2];
            stream.read_exact(&mut addr).await?;
        }
        _ => anyhow::bail!("SOCKS5 proxy returned unknown address type: {}", resp[3]),
    }

    Ok(stream)
}
