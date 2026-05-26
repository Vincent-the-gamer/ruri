//! Metrics collection for network monitoring and token usage tracking.
//!
//! This module provides time-series data collection for:
//! - Provider API request counts
//! - Network traffic (bytes in/out)
//! - Token consumption (per provider)
//!
//! Data is stored in-memory with a configurable retention window (default: 7 days).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of individual records to keep in memory.
/// With typical usage patterns, this covers ~7 days of data.
const DEFAULT_MAX_RECORDS: usize = 50_000;

/// A single data point with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: i64,
    /// The metric value.
    pub value: f64,
}

/// A single data point for time-series charts (matches AstrBot's format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    /// Unix timestamp in seconds.
    pub timestamp: i64,
    /// The metric value at this timestamp.
    pub value: f64,
}

/// Request count metric record.
#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp_ms: i64,
}

/// Traffic metric record.
#[derive(Debug, Clone)]
struct TrafficRecord {
    timestamp_ms: i64,
    bytes_sent: u64,
    bytes_received: u64,
}

/// Source of token consumption.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenSource {
    /// WebUI chat (debug session).
    DebugSession,
    /// Config profile with the profile name.
    Profile(String),
    /// ACP protocol session.
    Acp,
}

impl std::fmt::Display for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenSource::DebugSession => write!(f, "debug_session"),
            TokenSource::Profile(name) => write!(f, "profile:{name}"),
            TokenSource::Acp => write!(f, "acp"),
        }
    }
}

/// Token consumption metric record.
#[derive(Debug, Clone)]
struct TokenRecord {
    timestamp_ms: i64,
    provider_name: String,
    source: Option<TokenSource>,
    prompt_tokens: u64,
    completion_tokens: u64,
}

/// Metrics collector that stores time-series data in memory.
pub struct MetricsCollector {
    /// Request count records.
    requests: Vec<RequestRecord>,
    /// Traffic records.
    traffic: Vec<TrafficRecord>,
    /// Token usage records.
    tokens: Vec<TokenRecord>,
    /// Maximum number of records of each type.
    max_records: usize,
    /// Broadcast sender to notify WebSocket clients of updates.
    update_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl std::fmt::Debug for MetricsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsCollector")
            .field("requests", &self.requests.len())
            .field("traffic", &self.traffic.len())
            .field("tokens", &self.tokens.len())
            .field("max_records", &self.max_records)
            .finish()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            requests: Vec::new(),
            traffic: Vec::new(),
            tokens: Vec::new(),
            max_records: DEFAULT_MAX_RECORDS,
            update_tx: None,
        }
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the broadcast channel for notifying WebSocket clients of updates.
    pub fn set_update_channel(&mut self, tx: tokio::sync::broadcast::Sender<()>) {
        self.update_tx = Some(tx);
    }

    /// Notify subscribers that metrics have been updated.
    fn notify_update(&self) {
        if let Some(ref tx) = self.update_tx {
            let _ = tx.send(());
        }
    }

    /// Record a provider API request.
    pub fn record_request(&mut self, _provider_name: &str) {
        let now = Utc::now().timestamp_millis();
        self.requests.push(RequestRecord { timestamp_ms: now });
        self.trim_if_needed();
        self.notify_update();
    }

    /// Record network traffic (bytes sent and received).
    pub fn record_traffic(&mut self, bytes_sent: u64, bytes_received: u64) {
        let now = Utc::now().timestamp_millis();
        self.traffic.push(TrafficRecord {
            timestamp_ms: now,
            bytes_sent,
            bytes_received,
        });
        self.trim_if_needed();
        self.notify_update();
    }

    /// Record token usage for a provider with explicit source (debug_session / profile / acp).
    pub fn record_tokens_with_source(
        &mut self,
        provider_name: &str,
        _model: Option<&str>,
        prompt_tokens: u64,
        completion_tokens: u64,
        source: Option<TokenSource>,
    ) {
        if prompt_tokens == 0 && completion_tokens == 0 {
            return; // Don't record empty token usage
        }
        let now = Utc::now().timestamp_millis();
        self.tokens.push(TokenRecord {
            timestamp_ms: now,
            provider_name: provider_name.to_string(),
            source,
            prompt_tokens,
            completion_tokens,
        });
        self.trim_if_needed();
        self.notify_update();
    }

    fn trim_if_needed(&mut self) {
        if self.requests.len() > self.max_records {
            let excess = self.requests.len() - self.max_records;
            self.requests.drain(0..excess);
        }
        if self.traffic.len() > self.max_records {
            let excess = self.traffic.len() - self.max_records;
            self.traffic.drain(0..excess);
        }
        if self.tokens.len() > self.max_records {
            let excess = self.tokens.len() - self.max_records;
            self.tokens.drain(0..excess);
        }
    }

    /// Get the cutoff timestamp (in milliseconds) for a given number of days.
    fn cutoff_ms(days: u32) -> i64 {
        let now = Utc::now().timestamp_millis();
        now - (days as i64 * 24 * 60 * 60 * 1000)
    }

    // ─── Request metrics ───────────────────────────────────────────

    /// Get request count time series for the given number of days.
    /// Buckets data into intervals (more granular for shorter ranges).
    pub fn get_request_time_series(&self, days: u32) -> Vec<MetricPoint> {
        let cutoff = Self::cutoff_ms(days);
        let interval_ms = self.bucket_interval_ms(days);

        // Filter records within the time range
        let filtered: Vec<i64> = self
            .requests
            .iter()
            .filter(|r| r.timestamp_ms >= cutoff)
            .map(|r| r.timestamp_ms)
            .collect();

        if filtered.is_empty() {
            return Vec::new();
        }

        self.bucket_timestamps(&filtered, cutoff, interval_ms)
    }

    /// Get total request count in the given time range.
    pub fn get_total_requests(&self, days: u32) -> u64 {
        let cutoff = Self::cutoff_ms(days);
        self.requests
            .iter()
            .filter(|r| r.timestamp_ms >= cutoff)
            .count() as u64
    }

    // ─── Traffic metrics ───────────────────────────────────────────

    /// Get traffic time series (bytes in, bytes out) for the given number of days.
    pub fn get_traffic_time_series(&self, days: u32) -> (Vec<MetricPoint>, Vec<MetricPoint>) {
        let cutoff = Self::cutoff_ms(days);
        let interval_ms = self.bucket_interval_ms(days);

        let filtered: Vec<&TrafficRecord> = self
            .traffic
            .iter()
            .filter(|r| r.timestamp_ms >= cutoff)
            .collect();

        if filtered.is_empty() {
            return (Vec::new(), Vec::new());
        }

        self.bucket_traffic(&filtered, cutoff, interval_ms)
    }

    /// Get total traffic in the given time range.
    pub fn get_total_traffic(&self, days: u32) -> (u64, u64) {
        let cutoff = Self::cutoff_ms(days);
        let mut total_in: u64 = 0;
        let mut total_out: u64 = 0;
        for r in &self.traffic {
            if r.timestamp_ms >= cutoff {
                total_in += r.bytes_received;
                total_out += r.bytes_sent;
            }
        }
        (total_in, total_out)
    }

    // ─── Token metrics ─────────────────────────────────────────────

    /// Get token consumption time series for the given number of days.
    /// Returns a list of time series, one per provider.
    pub fn get_token_time_series(&self, days: u32) -> Vec<ProviderTokenSeries> {
        let cutoff = Self::cutoff_ms(days);
        let interval_ms = self.bucket_interval_ms(days);

        let filtered: Vec<&TokenRecord> = self
            .tokens
            .iter()
            .filter(|r| r.timestamp_ms >= cutoff)
            .collect();

        if filtered.is_empty() {
            return Vec::new();
        }

        // Group by provider
        let mut provider_data: HashMap<String, Vec<(i64, u64)>> = HashMap::new();
        for r in &filtered {
            let entry = provider_data.entry(r.provider_name.clone()).or_default();
            entry.push((r.timestamp_ms, r.prompt_tokens + r.completion_tokens));
        }

        provider_data
            .into_iter()
            .map(|(name, points)| {
                let total: u64 = points.iter().map(|(_, v)| v).sum();
                let buckets = Self::bucket_values(&points, cutoff, interval_ms);
                ProviderTokenSeries {
                    provider_name: name,
                    total_tokens: total,
                    points: buckets
                        .into_iter()
                        .map(|p| TimeSeriesPoint {
                            timestamp: p.timestamp_ms / 1000,
                            value: p.value,
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// Get total token usage in the given time range.
    pub fn get_total_tokens(&self, days: u32) -> u64 {
        let cutoff = Self::cutoff_ms(days);
        self.tokens
            .iter()
            .filter(|r| r.timestamp_ms >= cutoff)
            .map(|r| r.prompt_tokens + r.completion_tokens)
            .sum()
    }

    /// Get token usage breakdown by provider for the given time range.
    pub fn get_tokens_by_provider(&self, days: u32) -> Vec<ProviderTokenTotal> {
        let cutoff = Self::cutoff_ms(days);
        let mut map: HashMap<String, u64> = HashMap::new();
        for r in &self.tokens {
            if r.timestamp_ms >= cutoff {
                *map.entry(r.provider_name.clone()).or_default() +=
                    r.prompt_tokens + r.completion_tokens;
            }
        }
        map.into_iter()
            .map(|(name, tokens)| ProviderTokenTotal {
                provider_name: name,
                tokens,
            })
            .collect()
    }

    /// Get token usage breakdown by source for the given time range.
    pub fn get_tokens_by_source(&self, days: u32) -> Vec<SourceTokenTotal> {
        let cutoff = Self::cutoff_ms(days);
        let mut map: HashMap<String, u64> = HashMap::new();
        for r in &self.tokens {
            if r.timestamp_ms >= cutoff {
                let source_key = r
                    .source
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                *map.entry(source_key).or_default() += r.prompt_tokens + r.completion_tokens;
            }
        }
        map.into_iter()
            .map(|(source, tokens)| SourceTokenTotal { source, tokens })
            .collect()
    }

    /// Get summary stats for the given time range.
    pub fn get_summary(&self, days: u32) -> MetricsSummary {
        let (total_in, total_out) = self.get_total_traffic(days);
        let total_requests = self.get_total_requests(days);
        let total_tokens = self.get_total_tokens(days);
        let tokens_by_provider = self.get_tokens_by_provider(days);
        let tokens_by_source = self.get_tokens_by_source(days);

        MetricsSummary {
            days,
            total_requests,
            total_traffic_in: total_in,
            total_traffic_out: total_out,
            total_tokens,
            tokens_by_provider,
            tokens_by_source,
        }
    }

    // ─── Bucketing helpers ─────────────────────────────────────────

    /// Determine the bucket interval in milliseconds based on the time range.
    fn bucket_interval_ms(&self, days: u32) -> i64 {
        match days {
            1 => 10 * 60 * 1000,     // 10 minutes for 1 day
            3 => 30 * 60 * 1000,     // 30 minutes for 3 days
            7 => 2 * 60 * 60 * 1000, // 2 hours for 7 days
            _ => 60 * 60 * 1000,     // 1 hour default
        }
    }

    /// Bucket timestamps into count-based MetricPoints.
    fn bucket_timestamps(
        &self,
        timestamps: &[i64],
        cutoff: i64,
        interval_ms: i64,
    ) -> Vec<MetricPoint> {
        if timestamps.is_empty() {
            return Vec::new();
        }

        let num_buckets = ((Utc::now().timestamp_millis() - cutoff) / interval_ms + 1) as usize;
        let mut buckets = vec![0u64; num_buckets];

        for ts in timestamps {
            let idx = ((*ts - cutoff) / interval_ms) as usize;
            if idx < num_buckets {
                buckets[idx] += 1;
            }
        }

        buckets
            .into_iter()
            .enumerate()
            .map(|(i, count)| MetricPoint {
                timestamp_ms: cutoff + (i as i64 * interval_ms),
                value: count as f64,
            })
            .collect()
    }

    /// Bucket traffic records into (bytes_in, bytes_out) MetricPoints.
    fn bucket_traffic(
        &self,
        records: &[&TrafficRecord],
        cutoff: i64,
        interval_ms: i64,
    ) -> (Vec<MetricPoint>, Vec<MetricPoint>) {
        if records.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let num_buckets = ((Utc::now().timestamp_millis() - cutoff) / interval_ms + 1) as usize;
        let mut buckets_in = vec![0u64; num_buckets];
        let mut buckets_out = vec![0u64; num_buckets];

        for r in records {
            let idx = ((r.timestamp_ms - cutoff) / interval_ms) as usize;
            if idx < num_buckets {
                buckets_in[idx] += r.bytes_received;
                buckets_out[idx] += r.bytes_sent;
            }
        }

        let points_in: Vec<MetricPoint> = buckets_in
            .into_iter()
            .enumerate()
            .map(|(i, bytes)| MetricPoint {
                timestamp_ms: cutoff + (i as i64 * interval_ms),
                value: bytes as f64,
            })
            .collect();

        let points_out: Vec<MetricPoint> = buckets_out
            .into_iter()
            .enumerate()
            .map(|(i, bytes)| MetricPoint {
                timestamp_ms: cutoff + (i as i64 * interval_ms),
                value: bytes as f64,
            })
            .collect();

        (points_in, points_out)
    }

    /// Bucket (timestamp, value) pairs into MetricPoints.
    fn bucket_values(points: &[(i64, u64)], cutoff: i64, interval_ms: i64) -> Vec<MetricPoint> {
        if points.is_empty() {
            return Vec::new();
        }

        let num_buckets = ((Utc::now().timestamp_millis() - cutoff) / interval_ms + 1) as usize;
        let mut buckets = vec![0u64; num_buckets];

        for (ts, value) in points {
            let idx = ((*ts - cutoff) / interval_ms) as usize;
            if idx < num_buckets {
                buckets[idx] += value;
            }
        }

        buckets
            .into_iter()
            .enumerate()
            .map(|(i, total)| MetricPoint {
                timestamp_ms: cutoff + (i as i64 * interval_ms),
                value: total as f64,
            })
            .collect()
    }
}

// ─── API response types ────────────────────────────────────────────

/// Token time series for a single provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenSeries {
    pub provider_name: String,
    pub total_tokens: u64,
    pub points: Vec<TimeSeriesPoint>,
}

/// Token total for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTokenTotal {
    pub provider_name: String,
    pub tokens: u64,
}

/// Token total for a source (debug_session / profile / acp).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTokenTotal {
    pub source: String,
    pub tokens: u64,
}

/// Summary of all metrics for a time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub days: u32,
    pub total_requests: u64,
    pub total_traffic_in: u64,
    pub total_traffic_out: u64,
    pub total_tokens: u64,
    pub tokens_by_provider: Vec<ProviderTokenTotal>,
    pub tokens_by_source: Vec<SourceTokenTotal>,
}
