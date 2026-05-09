//! Chat platform adapters for Ruri.
//!
//! This module provides a pluggable platform abstraction layer so that
//! each configuration file can correspond to one independent adapter
//! instance that talks to a specific chat platform (DingTalk, Slack,
//! Discord, etc.).
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐    PlatformEvent     ┌───────────────┐
//! │  DingTalk    │ ────(mpsc channel)──► │  Platform     │
//! │  Adapter     │                       │  Manager      │
//! │  (Stream WS) │ ◄── REST API ─────── │               │
//! └──────────────┘                       └───────┬───────┘
//!                                                 │
//!                                           ┌─────▼──────┐
//!                                           │   Agent     │
//!                                           │  (Runner)   │
//!                                           └────────────┘
//! ```
//!
//! # Adding a new platform
//!
//! 1. Create a new module under `src/platform/<name>/`
//! 2. Implement the [`Platform`] trait
//! 3. Register it in [`PlatformManager::build_adapter`]
//! 4. Add config parsing support

pub mod dingtalk;
pub mod discord;
pub mod manager;
pub mod trait_def;
pub mod types;
pub mod weixin_oc;

pub use manager::PlatformManager;
pub use trait_def::PlatformEvent;
