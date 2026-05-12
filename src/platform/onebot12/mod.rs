//! OneBot v12 standard platform adapter.
//!
//! Implements the [OneBot v12](https://12.onebot.dev/) standard for
//! universal chat bot interface, supporting:
//! - HTTP action server
//! - HTTP Webhook event push
//! - Forward WebSocket server
//! - Reverse WebSocket client
//!
//! # Configuration
//!
//! ```yaml
//! platforms:
//!   - type: onebot12
//!     id: my-qq-bot
//!     platform: "qq"
//!     self_user_id: "123456"
//!     access_token: "mytoken"
//!     http:
//!       host: "0.0.0.0"
//!       port: 6700
//!       event_enabled: true
//!       event_buffer_size: 0
//!     ws:
//!       host: "0.0.0.0"
//!       port: 6701
//! ```
//!
//! # Adding a new platform
//!
//! 1. Create a new module under `src/platform/<name>/`
//! 2. Implement the [`Platform`] trait
//! 3. Register it in [`PlatformManager::build_adapter`]
//! 4. Add config parsing support

pub mod adapter;
pub mod config;
pub mod reverse_ws;
pub mod server;
pub mod types;

pub use adapter::OneBot12Adapter;
