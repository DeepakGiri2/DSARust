//! Recording traces with the Rhai engine, safely and cheaply.
//!
//! A trace is pure CPU (the script interpreter), so it never runs on the async
//! executor: it goes to the blocking pool behind a semaphore sized to the
//! machine, which keeps a burst of custom-input requests from starving every
//! other request on the task. The engine's own operation budget
//! (`dsa-content`'s `set_max_operations`) bounds each run.
//!
//! Results are deterministic functions of (content version, slug, input), so
//! they are cached by exactly that. The default input's trace is what every
//! first visit asks for; it is computed once per task and served from memory
//! (and from the CDN in front of it).

use crate::content::ContentStore;
use crate::dto;
use crate::error::{ApiError, ApiResult};
use bytes::Bytes;
use dsa_core::problem::{validate_inputs, InputMap};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Wall-clock ceiling for one trace. The operation budget normally stops a
/// runaway script well before this; the timeout is the backstop.
const TRACE_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone)]
pub struct TraceService {
    content: Arc<ContentStore>,
    cache: moka::future::Cache<[u8; 32], Bytes>,
    permits: Arc<Semaphore>,
}

impl TraceService {
    pub fn new(content: Arc<ContentStore>) -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);
        Self {
            content,
            cache: moka::future::Cache::builder()
                // Bounded by bytes, not entries: traces range from 2 KB to
                // several hundred.
                .weigher(|_k, v: &Bytes| v.len().min(u32::MAX as usize) as u32)
                .max_capacity(256 * 1024 * 1024)
                .time_to_idle(Duration::from_secs(6 * 3600))
                .build(),
            permits: Arc::new(Semaphore::new(cpus.max(1))),
        }
    }

    /// The trace for the problem's own default input, as serialized JSON.
    pub async fn default_trace(&self, slug: &str) -> ApiResult<Bytes> {
        let input = self
            .content
            .default_input(slug)
            .cloned()
            .ok_or_else(|| ApiError::not_found("problem"))?;
        self.trace(slug, input).await
    }

    /// Parse raw editor text per field with the manifest's own rules, exactly
    /// as the desktop's input panel does, collecting every problem at once.
    pub fn parse_fields(
        &self,
        slug: &str,
        fields: &BTreeMap<String, String>,
    ) -> ApiResult<InputMap> {
        let pack = self
            .content
            .pack(slug)
            .ok_or_else(|| ApiError::not_found("problem"))?;
        let mut input = InputMap::new();
        let mut errors = Vec::new();
        for f in &pack.meta.inputs {
            match fields.get(&f.name) {
                Some(text) => match f.parse(text) {
                    Ok(v) => {
                        input.insert(f.name.clone(), v);
                    }
                    Err(e) => errors.push(e),
                },
                None => errors.push(format!("missing input \"{}\"", f.display_label())),
            }
        }
        if !errors.is_empty() {
            return Err(ApiError::errors(errors));
        }
        Ok(input)
    }

    /// Validate and record, or answer from cache.
    pub async fn trace(&self, slug: &str, input: InputMap) -> ApiResult<Bytes> {
        let pack = self
            .content
            .pack(slug)
            .ok_or_else(|| ApiError::not_found("problem"))?;
        if !pack.has_script() {
            return Err(ApiError::NotFound(
                "this problem has no animation yet".into(),
            ));
        }
        // Unknown keys would be ignored by the script; refuse them so the
        // cache key cannot be inflated with junk.
        if let Some(k) = input
            .keys()
            .find(|k| !pack.meta.inputs.iter().any(|f| &f.name == *k))
        {
            return Err(ApiError::errors(vec![format!("unknown input \"{k}\"")]));
        }
        let mut errors = validate_inputs(&pack.meta.inputs, &input);
        if !errors.is_empty() {
            return Err(ApiError::errors(std::mem::take(&mut errors)));
        }

        let key = self.key(slug, &input);
        if let Some(hit) = self.cache.get(&key).await {
            metrics::counter!("dsa_trace_cache_hits_total").increment(1);
            return Ok(hit);
        }
        metrics::counter!("dsa_trace_cache_misses_total").increment(1);

        let _permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ApiError::Unavailable("tracer shutting down".into()))?;
        let content = self.content.clone();
        let slug_owned = slug.to_string();
        let started = std::time::Instant::now();
        let job = tokio::task::spawn_blocking(move || -> ApiResult<Bytes> {
            if let Some(msg) = content.lib.validate(&slug_owned, &input) {
                return Err(ApiError::errors(vec![msg]));
            }
            let trace = content
                .lib
                .trace(&slug_owned, &input)
                .map_err(|e| ApiError::errors(vec![e.to_string()]))?;
            let body = serde_json::to_vec(&dto::TraceResponse {
                input: &input,
                trace: &trace,
            })
            .map_err(ApiError::internal)?;
            Ok(Bytes::from(body))
        });
        let bytes = match tokio::time::timeout(TRACE_TIMEOUT, job).await {
            Ok(Ok(res)) => res?,
            Ok(Err(join)) => return Err(ApiError::internal(format!("trace task failed: {join}"))),
            Err(_) => {
                return Err(ApiError::errors(vec![
                    "this input takes too long to animate — try a smaller one".into(),
                ]))
            }
        };
        metrics::histogram!("dsa_trace_seconds").record(started.elapsed().as_secs_f64());
        self.cache.insert(key, bytes.clone()).await;
        Ok(bytes)
    }

    fn key(&self, slug: &str, input: &InputMap) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.content.version.as_bytes());
        h.update([0]);
        h.update(slug.as_bytes());
        h.update([0]);
        // InputMap is a BTreeMap, so this serialization is canonical.
        h.update(serde_json::to_vec(input).unwrap_or_default());
        h.finalize().into()
    }
}
