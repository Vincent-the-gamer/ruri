//! Stdio transport for MCP
//!
//! This module provides stdio based transport for communicating with local MCP servers.

use super::McpTransport;
use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, TransportConfig};
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use tracing::debug;

/// Stdio transport for MCP
pub struct StdioMcpTransport {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl StdioMcpTransport {
    /// Create a new stdio MCP transport from configuration
    pub fn new(config: &TransportConfig) -> Result<Self> {
        match config {
            TransportConfig::Stdio { command, args, env } => {
                let mut cmd = Command::new(command);

                if let Some(args) = args {
                    cmd.args(args);
                }

                if let Some(env_vars) = env {
                    for (key, value) in env_vars {
                        cmd.env(key, value);
                    }
                }

                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let mut child = cmd.spawn().context("Failed to start MCP server process")?;

                let stdin = child.stdin.take().expect("Failed to open stdin");
                let stdout = child.stdout.take().expect("Failed to open stdout");

                debug!("Started stdio transport for command: {}", command);

                Ok(Self {
                    child: Some(child),
                    stdin: Some(stdin),
                    stdout: BufReader::new(stdout),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Invalid transport configuration for stdio transport"
            )),
        }
    }
}

#[async_trait::async_trait]
impl McpTransport for StdioMcpTransport {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Use a separate thread to read stdout to avoid blocking the async runtime
        let mut stdin = self
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Stdin is closed"))?;
        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC request")?;

        debug!("Sending stdio request: {}", request_json);

        // Write to stdin in a blocking manner
        writeln!(stdin, "{}", request_json).context("Failed to write to MCP server stdin")?;
        stdin.flush().context("Failed to flush MCP server stdin")?;

        self.stdin = Some(stdin);

        // Read response from stdout
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .context("Failed to read response from MCP server")?;

        debug!("Received stdio response: {}", line.trim());

        let response: JsonRpcResponse =
            serde_json::from_str(&line.trim()).context("Failed to parse JSON-RPC response")?;

        Ok(response)
    }

    async fn send_notification(&mut self, request: JsonRpcRequest) -> Result<()> {
        let mut stdin = self
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Stdin is closed"))?;

        let request_json =
            serde_json::to_string(&request).context("Failed to serialize JSON-RPC notification")?;

        debug!("Sending stdio notification: {}", request_json);

        writeln!(stdin, "{}", request_json).context("Failed to write to MCP server stdin")?;
        stdin.flush().context("Failed to flush MCP server stdin")?;

        self.stdin = Some(stdin);

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        debug!("Shutting down stdio transport");

        // Close stdin
        self.stdin = None;

        // Wait for child process to finish
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }

        debug!("Stdio transport shut down");

        Ok(())
    }
}
