//! Shared WebSocket connection through HTTP/SOCKS5 proxy.
//!
//! This module provides utilities for establishing WebSocket connections
//! through HTTP CONNECT or SOCKS5 proxies, used by platform adapters
//! that need proxy support (e.g., DingTalk Stream, Discord Gateway).

use anyhow::anyhow;
use base64::Engine;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, client_async_with_config};

/// The type of WebSocket stream returned by the proxy connection functions.
pub type ProxiedWsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Connect to a WebSocket endpoint, optionally through a proxy.
///
/// If `proxy_url` is `None` or empty, a direct connection is made.
/// For `wss://` URLs the appropriate TLS handshake is performed after
/// the proxy tunnel (or directly) is established.
///
/// Supported proxy schemes:
/// - `http://` / `https://` — HTTP CONNECT tunnel
/// - `socks5://` / `socks5h://` — SOCKS5 proxy
pub async fn connect_ws_with_proxy(
    ws_url: &str,
    proxy_url: Option<&str>,
) -> anyhow::Result<ProxiedWsStream> {
    let request = ws_url.into_client_request()?;
    let host = request
        .uri()
        .host()
        .ok_or_else(|| anyhow!("No host in WebSocket URL"))?
        .to_string();
    let port = request
        .uri()
        .port_u16()
        .unwrap_or(if ws_url.starts_with("wss://") {
            443
        } else {
            80
        });
    let is_tls = ws_url.starts_with("wss://");

    match proxy_url {
        Some(proxy) if !proxy.is_empty() => {
            tracing::info!(proxy = %proxy, host = %host, port = %port, "Connecting WebSocket via proxy");
            if proxy.starts_with("socks5://") || proxy.starts_with("socks5h://") {
                connect_ws_via_socks5_proxy(request, &host, port, is_tls, proxy).await
            } else {
                connect_ws_via_http_proxy(request, &host, port, is_tls, proxy).await
            }
        }
        _ => {
            // Direct connection (no proxy)
            let (ws, _) =
                tokio_tungstenite::connect_async_tls_with_config(ws_url, None, false, None)
                    .await
                    .map_err(|e| anyhow!("WebSocket connect failed: {}", e))?;
            Ok(ws)
        }
    }
}

/// Connect WebSocket through an HTTP CONNECT proxy.
async fn connect_ws_via_http_proxy(
    request: tokio_tungstenite::tungstenite::http::Request<()>,
    host: &str,
    port: u16,
    is_tls: bool,
    proxy_url: &str,
) -> anyhow::Result<ProxiedWsStream> {
    // Parse proxy URL
    let proxy_uri: url::Url = proxy_url.parse()?;
    let proxy_host = proxy_uri
        .host_str()
        .ok_or_else(|| anyhow!("No host in proxy URL"))?
        .to_string();
    let proxy_port = proxy_uri
        .port_or_known_default()
        .ok_or_else(|| anyhow!("No port in proxy URL"))?;

    // Connect to the proxy
    let mut stream = TcpStream::connect((&*proxy_host, proxy_port)).await?;

    // Send CONNECT request (with optional proxy auth)
    let mut connect_req = format!(
        "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
        host, port, host, port
    );

    // Add Proxy-Authorization if proxy URL has credentials
    let username = proxy_uri.username();
    if !username.is_empty() {
        let password = proxy_uri.password().unwrap_or("");
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
        connect_req.push_str(&format!("Proxy-Authorization: basic {}\r\n", credentials));
    }
    connect_req.push_str("\r\n");

    stream.write_all(connect_req.as_bytes()).await?;
    stream.flush().await?;

    // Read proxy response
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    // Check for 200 OK
    if !response_line.contains("200") {
        anyhow::bail!(
            "HTTP CONNECT proxy returned non-200: {}",
            response_line.trim()
        );
    }

    // Consume remaining headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
    }

    // Get the underlying stream back from BufReader
    let stream = reader.into_inner();

    if is_tls {
        // Perform TLS handshake over the tunnel
        let tls_connector = native_tls::TlsConnector::new()?;
        let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
            .connect(host, stream)
            .await?;

        // WebSocket upgrade over TLS stream (TLS already done, just do WS handshake)
        let ws_stream = MaybeTlsStream::NativeTls(tls_stream);
        let (ws, _) = client_async_with_config(request, ws_stream, None)
            .await
            .map_err(|e| anyhow!("WebSocket handshake over proxy tunnel failed: {}", e))?;
        Ok(ws)
    } else {
        // Plain WebSocket upgrade
        let ws_stream = MaybeTlsStream::Plain(stream);
        let (ws, _) = client_async_with_config(request, ws_stream, None)
            .await
            .map_err(|e| anyhow!("WebSocket handshake over proxy tunnel failed: {}", e))?;
        Ok(ws)
    }
}

/// Connect WebSocket through a SOCKS5 proxy.
async fn connect_ws_via_socks5_proxy(
    request: tokio_tungstenite::tungstenite::http::Request<()>,
    host: &str,
    port: u16,
    is_tls: bool,
    proxy_url: &str,
) -> anyhow::Result<ProxiedWsStream> {
    // Parse proxy URL for host/port/auth
    let proxy_uri: url::Url = proxy_url.parse()?;
    let proxy_host = proxy_uri
        .host_str()
        .ok_or_else(|| anyhow!("No host in proxy URL"))?
        .to_string();
    let proxy_port = proxy_uri
        .port_or_known_default()
        .ok_or_else(|| anyhow!("No port in proxy URL"))?;

    // Connect to SOCKS5 proxy
    let proxy_stream = TcpStream::connect((&*proxy_host, proxy_port)).await?;

    // Build SOCKS5 authentication
    let (username, password) = if proxy_uri.username().is_empty() {
        (String::new(), String::new())
    } else {
        (
            proxy_uri.username().to_string(),
            proxy_uri.password().unwrap_or("").to_string(),
        )
    };

    // Perform SOCKS5 handshake
    let stream = socks5_handshake(proxy_stream, host, port, &username, &password).await?;

    if is_tls {
        let tls_connector = native_tls::TlsConnector::new()?;
        let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
            .connect(host, stream)
            .await?;

        let ws_stream = MaybeTlsStream::NativeTls(tls_stream);
        let (ws, _) = client_async_with_config(request, ws_stream, None)
            .await
            .map_err(|e| anyhow!("WebSocket handshake over SOCKS5 proxy failed: {}", e))?;
        Ok(ws)
    } else {
        let ws_stream = MaybeTlsStream::Plain(stream);
        let (ws, _) = client_async_with_config(request, ws_stream, None)
            .await
            .map_err(|e| anyhow!("WebSocket handshake over SOCKS5 proxy failed: {}", e))?;
        Ok(ws)
    }
}

/// Perform a SOCKS5 handshake on a TCP stream.
///
/// Supports both no-auth and username/password authentication.
async fn socks5_handshake(
    mut stream: TcpStream,
    target_host: &str,
    target_port: u16,
    username: &str,
    password: &str,
) -> anyhow::Result<TcpStream> {
    // Step 1: Client greeting
    if username.is_empty() {
        // No auth
        stream.write_all(&[0x05, 0x01, 0x00]).await?; // VER, NMETHODS, NO AUTH
    } else {
        // Username/password auth
        stream.write_all(&[0x05, 0x02, 0x00, 0x02]).await?; // VER, NMETHODS, NO AUTH, USERNAME/PASSWORD
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
        // Username/password sub-negotiation
        let ulen = username.len() as u8;
        let plen = password.len() as u8;
        let mut auth_req = Vec::with_capacity(3 + username.len() + password.len());
        auth_req.push(0x01); // Sub-negotiation version
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
            // IPv4
            let mut addr = [0u8; 4 + 2];
            stream.read_exact(&mut addr).await?;
        }
        0x03 => {
            // Domain
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut addr = vec![0u8; len_buf[0] as usize + 2];
            stream.read_exact(&mut addr).await?;
        }
        0x04 => {
            // IPv6
            let mut addr = [0u8; 16 + 2];
            stream.read_exact(&mut addr).await?;
        }
        _ => anyhow::bail!("SOCKS5 proxy returned unknown address type: {}", resp[3]),
    }

    Ok(stream)
}
