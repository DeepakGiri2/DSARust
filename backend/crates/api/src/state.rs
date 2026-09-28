//! Shared, immutable-after-boot application state.
//!
//! Everything here is either read-only (config, content) or internally
//! synchronized (pools, caches, clients), so a clone is an `Arc` bump and every
//! handler on every task sees the same thing. Nothing request-specific lives
//! here — which is what makes a task disposable and the fleet horizontally
//! scalable.

use crate::ai::Assistant;
use crate::config::Config;
use crate::content::ContentStore;
use crate::mail::Mailer;
use crate::ratelimit::RateLimiter;
use crate::runner::Runner;
use crate::traces::TraceService;
use sqlx::PgPool;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub struct Inner {
    pub cfg: Config,
    pub db: PgPool,
    pub content: Arc<ContentStore>,
    pub traces: TraceService,
    pub runner: Runner,
    pub mailer: Box<dyn Mailer>,
    pub limiter: Box<dyn RateLimiter>,
    pub ai: Option<Assistant>,
    /// One pooled HTTPS client for OAuth, Stripe and model providers.
    pub http: reqwest::Client,
    pub redis: Option<redis::aio::ConnectionManager>,
}

impl AppState {
    pub fn new(inner: Inner) -> Self {
        Self(Arc::new(inner))
    }
}

impl Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}

impl Inner {
    /// Absolute URL on the SPA's origin (email links, OAuth redirects).
    pub fn public_link(&self, path_and_query: &str) -> String {
        let base = self.cfg.public_url.as_str().trim_end_matches('/');
        format!("{base}{path_and_query}")
    }
}
