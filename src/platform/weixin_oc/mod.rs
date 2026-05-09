//! WeChat ClawBot (openclaw-weixin) platform adapter for Ruri.
//!
//! This adapter implements the WeChat personal account (ClawBot) protocol,
//! supporting QR code login and long-polling message receive/send.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────┐   PlatformEvent   ┌───────────────┐
//! │  WeChat ClawBot   │ ──(mpsc channel)─►│  Platform     │
//! │  Adapter          │                    │  Manager      │
//! │  (HTTP long-poll) │ ◄── send_text ─── │               │
//! └───────────────────┘                    └───────┬───────┘
//!                                                  │
//!                                            ┌─────▼──────┐
//!                                            │   Agent     │
//!                                            │  (Runner)   │
//!                                            └────────────┘
//! ```
//!
//! # Components
//!
//! - `config` — Configuration struct ([`WeixinOcConfig`])
//! - `types` — API type definitions matching the WeChat iLink protocol
//! - `api` — HTTP client for all iLink endpoints

//! - `adapter` — [`Platform`] trait implementation

pub mod adapter;
pub mod api;

pub mod config;
pub mod types;

pub use adapter::WeixinOcAdapter;
