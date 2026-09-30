//! A small JSON-RPC client aimed at free public endpoints.
//!
//! Two things drive the design. Public RPCs cap `eth_getLogs` by block span and
//! by result count, so `get_logs` halves its range and retries when it is told
//! it asked for too much. And block timestamps are not in the log payload, so
//! they need a second call per block -- which is only affordable as a JSON-RPC
//! *batch*, one HTTP round-trip per hundred blocks instead of one per block.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug)]
pub enum RpcError {
    Http(String),
    /// An `error` object came back from the node.
    Node { code: i64, message: String },
    Decode(String),
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(e) => write!(f, "http: {e}"),
            Self::Node { code, message } => write!(f, "rpc {code}: {message}"),
            Self::Decode(e) => write!(f, "decode: {e}"),
        }
    }
}

impl RpcError {
    /// True when the node is refusing the *size* of the query rather than the
    /// query itself. Public endpoints phrase this many different ways and use
    /// several codes, so both are checked.
    pub fn is_range_too_large(&self) -> bool {
        let Self::Node { code, message } = self else {
            return false;
        };
        let m = message.to_lowercase();
        matches!(code, -32005 | -32062 | -32602)
            && (m.contains("range")
                || m.contains("more than")
                || m.contains("too many")
                || m.contains("limit")
                || m.contains("exceed")
                || m.contains("large")
                || m.contains("block span"))
    }

    /// Worth trying the identical request again after a pause.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Http(e) => {
                let e = e.to_lowercase();
                e.contains("timeout")
                    || e.contains("429")
                    || e.contains("502")
                    || e.contains("503")
                    || e.contains("504")
                    || e.contains("connection")
                    || e.contains("dns")
                    || e.contains("reset")
            }
            Self::Node { code, message } => {
                let m = message.to_lowercase();
                *code == -32005 && (m.contains("rate") || m.contains("many requests"))
                    || m.contains("capacity")
                    || m.contains("busy")
                    || m.contains("try again")
            }
            Self::Decode(_) => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "logIndex")]
    pub log_index: String,
    #[serde(default)]
    pub removed: bool,
}

impl Log {
    pub fn block(&self) -> Option<u64> {
        parse_quantity(&self.block_number)
    }

    pub fn index(&self) -> Option<u64> {
        parse_quantity(&self.log_index)
    }

    pub fn topic0(&self) -> Option<&str> {
        self.topics.first().map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub struct BlockMeta {
    pub number: u64,
    pub hash: String,
    pub timestamp: i64,
}

/// A hex `QUANTITY` (`"0x1b4"`) as used by every numeric field in the JSON-RPC
/// spec. Plain decimal is accepted too; some gateways return it.
pub fn parse_quantity(raw: &str) -> Option<u64> {
    match raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        Some(body) => u64::from_str_radix(body, 16).ok(),
        None => raw.parse::<u64>().ok(),
    }
}

pub fn quantity(n: u64) -> String {
    format!("0x{n:x}")
}

pub struct Rpc {
    http: reqwest::Client,
    url: String,
    max_retries: u32,
    backoff: Duration,
}

impl Rpc {
    pub fn new(url: String, timeout: Duration, max_retries: u32, backoff: Duration) -> Self {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();
        Self { http, url, max_retries, backoff }
    }

    async fn post(&self, body: &Value) -> Result<Value, RpcError> {
        let res = self
            .http
            .post(&self.url)
            .json(body)
            .send()
            .await
            .map_err(|e| RpcError::Http(e.to_string()))?;

        let status = res.status();
        if !status.is_success() {
            return Err(RpcError::Http(format!("status {status}")));
        }

        res.json::<Value>()
            .await
            .map_err(|e| RpcError::Decode(e.to_string()))
    }

    fn unwrap_response(v: &Value) -> Result<Value, RpcError> {
        if let Some(err) = v.get("error") {
            return Err(RpcError::Node {
                code: err.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            });
        }
        v.get("result")
            .cloned()
            .ok_or_else(|| RpcError::Decode("response has neither result nor error".into()))
    }

    /// One call, retried on transient failures with exponential backoff.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });

        let mut attempt = 0;
        loop {
            let outcome = match self.post(&body).await {
                Ok(v) => Self::unwrap_response(&v),
                Err(e) => Err(e),
            };

            match outcome {
                Ok(v) => return Ok(v),
                Err(e) if e.is_transient() && attempt < self.max_retries => {
                    tokio::time::sleep(self.backoff * 2_u32.pow(attempt)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Several calls in one HTTP request. Results come back keyed by `id`, which
    /// is not required to preserve order, so they are re-sorted into the order
    /// the callers asked for.
    pub async fn batch(&self, calls: &[(&str, Value)]) -> Result<Vec<Value>, RpcError> {
        if calls.is_empty() {
            return Ok(Vec::new());
        }

        let body = Value::Array(
            calls
                .iter()
                .enumerate()
                .map(|(i, (method, params))| {
                    json!({ "jsonrpc": "2.0", "id": i, "method": method, "params": params })
                })
                .collect(),
        );

        let mut attempt = 0;
        let raw = loop {
            match self.post(&body).await {
                Ok(v) => break v,
                Err(e) if e.is_transient() && attempt < self.max_retries => {
                    tokio::time::sleep(self.backoff * 2_u32.pow(attempt)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        };

        // A batch can be rejected wholesale, in which case a single error object
        // comes back instead of an array.
        let Some(items) = raw.as_array() else {
            return Err(Self::unwrap_response(&raw)
                .err()
                .unwrap_or_else(|| RpcError::Decode("batch did not return an array".into())));
        };

        let mut out = vec![Value::Null; calls.len()];
        for item in items {
            let Some(id) = item.get("id").and_then(Value::as_u64) else {
                continue;
            };
            if let Some(slot) = out.get_mut(id as usize) {
                // A failed member of a batch is left Null rather than sinking
                // the whole batch; the caller decides what a gap means.
                *slot = Self::unwrap_response(item).unwrap_or(Value::Null);
            }
        }
        Ok(out)
    }

    pub async fn block_number(&self) -> Result<u64, RpcError> {
        let v = self.call("eth_blockNumber", json!([])).await?;
        v.as_str()
            .and_then(parse_quantity)
            .ok_or_else(|| RpcError::Decode(format!("bad block number {v}")))
    }

    /// `eth_getLogs` over `[from, to]`, splitting the range in half whenever the
    /// node says the query was too big, down to a single block.
    pub async fn get_logs(
        &self,
        from: u64,
        to: u64,
        addresses: &[String],
        topic0: &[String],
    ) -> Result<Vec<Log>, RpcError> {
        let mut filter = json!({
            "fromBlock": quantity(from),
            "toBlock": quantity(to),
        });
        if !addresses.is_empty() {
            filter["address"] = json!(addresses);
        }
        if !topic0.is_empty() {
            filter["topics"] = json!([topic0]);
        }

        match self.call("eth_getLogs", json!([filter])).await {
            Ok(v) => serde_json::from_value::<Vec<Log>>(v)
                .map_err(|e| RpcError::Decode(format!("logs: {e}"))),
            Err(e) if e.is_range_too_large() && to > from => {
                let mid = from + (to - from) / 2;
                let mut left =
                    Box::pin(self.get_logs(from, mid, addresses, topic0)).await?;
                let right =
                    Box::pin(self.get_logs(mid + 1, to, addresses, topic0)).await?;
                left.extend(right);
                Ok(left)
            }
            Err(e) => Err(e),
        }
    }

    /// Same as `get_logs` but constrains an indexed argument, used to keep the
    /// collateral feed to transfers that touch a Polymarket contract.
    pub async fn get_logs_with_topic(
        &self,
        from: u64,
        to: u64,
        addresses: &[String],
        topic0: &[String],
        position: usize,
        values: &[String],
    ) -> Result<Vec<Log>, RpcError> {
        let mut topics: Vec<Value> = vec![json!(topic0)];
        while topics.len() < position {
            topics.push(Value::Null);
        }
        topics.push(json!(values));

        let mut filter = json!({
            "fromBlock": quantity(from),
            "toBlock": quantity(to),
            "topics": topics,
        });
        if !addresses.is_empty() {
            filter["address"] = json!(addresses);
        }

        match self.call("eth_getLogs", json!([filter])).await {
            Ok(v) => serde_json::from_value::<Vec<Log>>(v)
                .map_err(|e| RpcError::Decode(format!("logs: {e}"))),
            Err(e) if e.is_range_too_large() && to > from => {
                let mid = from + (to - from) / 2;
                let mut left = Box::pin(self.get_logs_with_topic(
                    from, mid, addresses, topic0, position, values,
                ))
                .await?;
                let right = Box::pin(self.get_logs_with_topic(
                    mid + 1, to, addresses, topic0, position, values,
                ))
                .await?;
                left.extend(right);
                Ok(left)
            }
            Err(e) => Err(e),
        }
    }

    /// Headers for many blocks in as few round-trips as possible.
    pub async fn blocks(&self, numbers: &[u64], chunk: usize) -> Vec<BlockMeta> {
        let mut out = Vec::with_capacity(numbers.len());

        for group in numbers.chunks(chunk.max(1)) {
            let calls: Vec<(&str, Value)> = group
                .iter()
                .map(|n| ("eth_getBlockByNumber", json!([quantity(*n), false])))
                .collect();

            let Ok(results) = self.batch(&calls).await else {
                continue;
            };

            for v in results {
                if let Some(meta) = block_meta(&v) {
                    out.push(meta);
                }
            }
        }

        out
    }

    pub async fn block(&self, number: u64) -> Result<Option<BlockMeta>, RpcError> {
        let v = self
            .call("eth_getBlockByNumber", json!([quantity(number), false]))
            .await?;
        Ok(block_meta(&v))
    }
}

fn block_meta(v: &Value) -> Option<BlockMeta> {
    Some(BlockMeta {
        number: parse_quantity(v.get("number")?.as_str()?)?,
        hash: v.get("hash")?.as_str()?.to_string(),
        timestamp: parse_quantity(v.get("timestamp")?.as_str()?)? as i64,
    })
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn quantities_round_trip() {
        assert_eq!(parse_quantity("0x1b4"), Some(436));
        assert_eq!(parse_quantity("0x0"), Some(0));
        assert_eq!(parse_quantity("436"), Some(436), "bare decimal is tolerated");
        assert_eq!(parse_quantity("0xzz"), None);
        assert_eq!(quantity(436), "0x1b4");
    }

    #[test]
    fn range_errors_are_told_apart_from_real_failures() {
        let too_big = RpcError::Node {
            code: -32005,
            message: "query returned more than 10000 results".into(),
        };
        assert!(too_big.is_range_too_large());

        let span = RpcError::Node {
            code: -32602,
            message: "eth_getLogs block range too large, max is 1000".into(),
        };
        assert!(span.is_range_too_large());

        let real = RpcError::Node { code: -32000, message: "execution reverted".into() };
        assert!(!real.is_range_too_large());
        assert!(!real.is_transient());

        assert!(RpcError::Http("operation timeout".into()).is_transient());
        assert!(RpcError::Http("status 429".into()).is_transient());
        assert!(!RpcError::Decode("bad json".into()).is_transient());
    }

    #[test]
    fn a_log_exposes_its_position() {
        let log: Log = serde_json::from_str(
            r#"{"address":"0xabc","topics":["0xt0"],"data":"0x",
                "blockNumber":"0x10","blockHash":"0xbh",
                "transactionHash":"0xtx","logIndex":"0x2"}"#,
        )
        .unwrap();
        assert_eq!((log.block(), log.index()), (Some(16), Some(2)));
        assert_eq!(log.topic0(), Some("0xt0"));
        assert!(!log.removed, "absent `removed` defaults to false");
    }

    #[test]
    fn block_meta_reads_a_header() {
        let v = json!({"number":"0x10","hash":"0xbh","timestamp":"0x655f1c00"});
        let m = block_meta(&v).unwrap();
        assert_eq!((m.number, m.hash.as_str()), (16, "0xbh"));
        assert_eq!(m.timestamp, 0x655f_1c00);
        assert!(block_meta(&json!({"number":"0x10"})).is_none());
    }
}
