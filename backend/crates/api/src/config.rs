//! Configuration, read once from the environment at startup.
//!
//! Every knob is an environment variable (documented in
//! `docs/platform/SERVICES.md`) because that is what ECS task definitions,
//! docker-compose and a developer's `.env` all speak. Parsing is strict: a
//! malformed value stops the process at boot with a message naming the
//! variable, rather than surfacing later as a confusing runtime failure.

use anyhow::{anyhow, bail, Context, Result};
use dsa_core::problem::Tier;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    Development,
    Production,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Clone, Debug)]
pub enum RunnerMode {
    /// Code execution is off; `Meta.features.runner` is false.
    Disabled,
    /// A `dsa-runner` in http mode (docker-compose, ECS).
    Http { url: Url, token: Option<String> },
    /// The runner as an AWS Lambda function, invoked with the task role.
    Lambda { function: String },
    /// Development only: run with the desktop harness in-process, on whatever
    /// toolchains this machine has. No sandbox at all, so it refuses to start
    /// in production.
    Local,
}

#[derive(Clone, Debug)]
pub enum MailMode {
    Log,
    Smtp { url: String },
    Ses,
}

#[derive(Clone, Debug)]
pub enum AiProviderConfig {
    None,
    Ollama { url: Url },
    Anthropic { api_key: String },
    Bedrock,
}

#[derive(Clone, Debug)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Clone, Debug)]
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub price_monthly: String,
    pub price_yearly: String,
}

/// Plan-dependent limits and product switches.
#[derive(Clone, Debug)]
pub struct Policy {
    pub premium_tiers: Vec<Tier>,
    pub runs_free_per_min: u32,
    pub runs_pro_per_min: u32,
    pub ai_free_daily: u32,
    pub ai_pro_daily: u32,
    pub require_verified_email: bool,
    pub signup_enabled: bool,
    pub profiles_per_account: usize,
    pub draft_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub env: Env,
    pub bind: SocketAddr,
    pub metrics_bind: Option<SocketAddr>,
    pub public_url: Url,
    /// Normalized `scheme://host[:port]` strings.
    pub allowed_origins: Vec<String>,
    pub content_dir: PathBuf,
    pub db: PgConnectOptions,
    pub db_max_connections: u32,
    pub redis_url: Option<String>,
    pub session_secret: Vec<u8>,
    pub session_ttl: Duration,
    pub cookie_secure: bool,
    pub run_migrations: bool,
    pub trusted_proxy_hops: usize,
    pub origin_verify_secret: Option<String>,
    pub runner: RunnerMode,
    pub runner_concurrency: usize,
    pub mail: MailMode,
    pub mail_from: String,
    pub ai: AiProviderConfig,
    pub ai_model: Option<String>,
    pub policy: Policy,
    pub admin_emails: Vec<String>,
    pub github: Option<OAuthClient>,
    pub google: Option<OAuthClient>,
    pub stripe: Option<StripeConfig>,
    pub aws_region: Option<String>,
    pub log_format: LogFormat,
}

impl Config {
    pub fn is_production(&self) -> bool {
        self.env == Env::Production
    }

    /// Read and validate the whole configuration.
    pub fn from_env() -> Result<Self> {
        let env = match var("DSA_ENV").as_deref() {
            None | Some("development") | Some("dev") => Env::Development,
            Some("production") | Some("prod") => Env::Production,
            Some(other) => bail!("DSA_ENV must be development or production, got {other:?}"),
        };
        let prod = env == Env::Production;

        let public_url: Url = parse_or("DSA_PUBLIC_URL", "http://localhost:5173")?;
        let mut allowed_origins = match var("DSA_ALLOWED_ORIGINS") {
            Some(list) => list
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(normalize_origin)
                .collect::<Result<Vec<_>>>()?,
            None => vec![],
        };
        let own = normalize_origin(public_url.as_str())?;
        if !allowed_origins.contains(&own) {
            allowed_origins.push(own);
        }

        let session_secret = match var("SESSION_SECRET") {
            Some(s) => decode_secret(&s).context("SESSION_SECRET")?,
            None if prod => bail!("SESSION_SECRET is required in production"),
            // A random per-process secret: sessions and CSRF tokens simply stop
            // validating across a restart, which is fine on a laptop.
            None => {
                use rand::RngCore;
                let mut b = vec![0u8; 32];
                rand::rngs::OsRng.fill_bytes(&mut b);
                b
            }
        };
        if session_secret.len() < 32 {
            bail!("SESSION_SECRET must be at least 32 bytes of entropy");
        }

        let runner = match var("RUNNER_MODE").as_deref() {
            None | Some("disabled") | Some("none") => RunnerMode::Disabled,
            Some("http") => RunnerMode::Http {
                url: parse_req("RUNNER_URL")?,
                token: var("RUNNER_TOKEN"),
            },
            Some("lambda") => RunnerMode::Lambda {
                function: var("RUNNER_LAMBDA_FUNCTION").ok_or_else(|| {
                    anyhow!("RUNNER_LAMBDA_FUNCTION is required for RUNNER_MODE=lambda")
                })?,
            },
            Some("local") if prod => {
                bail!("RUNNER_MODE=local executes code without a sandbox and is refused in production")
            }
            Some("local") => RunnerMode::Local,
            Some(other) => {
                bail!("RUNNER_MODE must be disabled, http, lambda or local, got {other:?}")
            }
        };

        let mail = match var("MAIL_MODE").as_deref() {
            None | Some("log") => MailMode::Log,
            Some("smtp") => MailMode::Smtp {
                url: var("SMTP_URL")
                    .ok_or_else(|| anyhow!("SMTP_URL is required for MAIL_MODE=smtp"))?,
            },
            Some("ses") => MailMode::Ses,
            Some(other) => bail!("MAIL_MODE must be log, smtp or ses, got {other:?}"),
        };

        let ai = match var("AI_PROVIDER").as_deref() {
            None | Some("none") => AiProviderConfig::None,
            Some("ollama") => AiProviderConfig::Ollama {
                url: parse_or("OLLAMA_URL", dsa_ai::DEFAULT_OLLAMA_URL)?,
            },
            Some("anthropic") => AiProviderConfig::Anthropic {
                api_key: var("ANTHROPIC_API_KEY").ok_or_else(|| {
                    anyhow!("ANTHROPIC_API_KEY is required for AI_PROVIDER=anthropic")
                })?,
            },
            Some("bedrock") => AiProviderConfig::Bedrock,
            Some(other) => {
                bail!("AI_PROVIDER must be none, ollama, anthropic or bedrock, got {other:?}")
            }
        };

        let stripe = match var("STRIPE_SECRET_KEY") {
            Some(secret_key) => Some(StripeConfig {
                secret_key,
                webhook_secret: var("STRIPE_WEBHOOK_SECRET").ok_or_else(|| {
                    anyhow!("STRIPE_WEBHOOK_SECRET is required with STRIPE_SECRET_KEY")
                })?,
                price_monthly: var("STRIPE_PRICE_MONTHLY").ok_or_else(|| {
                    anyhow!("STRIPE_PRICE_MONTHLY is required with STRIPE_SECRET_KEY")
                })?,
                price_yearly: var("STRIPE_PRICE_YEARLY").ok_or_else(|| {
                    anyhow!("STRIPE_PRICE_YEARLY is required with STRIPE_SECRET_KEY")
                })?,
            }),
            None => None,
        };

        let premium_tiers = match var("PREMIUM_TIERS") {
            Some(list) => list
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|t| {
                    serde_json::from_value::<Tier>(serde_json::Value::String(t.to_string()))
                        .map_err(|_| {
                            anyhow!("PREMIUM_TIERS: unknown tier {t:?} (50, 150, 250, extra)")
                        })
                })
                .collect::<Result<Vec<_>>>()?,
            None => vec![],
        };

        let log_format = match var("LOG_FORMAT").as_deref() {
            Some("json") => LogFormat::Json,
            Some("pretty") => LogFormat::Pretty,
            None if prod => LogFormat::Json,
            None => LogFormat::Pretty,
            Some(other) => bail!("LOG_FORMAT must be json or pretty, got {other:?}"),
        };

        let cookie_secure = match var("COOKIE_SECURE") {
            Some(v) => parse_bool("COOKIE_SECURE", &v)?,
            None => prod || public_url.scheme() == "https",
        };
        if prod && !cookie_secure {
            bail!("COOKIE_SECURE=false is refused in production");
        }
        if prod && public_url.scheme() != "https" {
            bail!("DSA_PUBLIC_URL must be https in production");
        }

        Ok(Self {
            env,
            bind: parse_or("DSA_BIND", "0.0.0.0:8080")?,
            metrics_bind: match var("DSA_METRICS_BIND") {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s.parse().context("DSA_METRICS_BIND")?),
                None => Some("0.0.0.0:9090".parse()?),
            },
            public_url,
            allowed_origins,
            content_dir: content_dir(),
            db: db_options(prod)?,
            db_max_connections: parse_or("DB_MAX_CONNECTIONS", "20")?,
            redis_url: redis_url()?,
            session_secret,
            session_ttl: Duration::from_secs(86_400 * parse_or::<u64>("SESSION_TTL_DAYS", "30")?),
            cookie_secure,
            run_migrations: flag("RUN_MIGRATIONS", false)?,
            trusted_proxy_hops: parse_or("TRUSTED_PROXY_HOPS", "0")?,
            origin_verify_secret: var("ORIGIN_VERIFY_SECRET"),
            runner,
            runner_concurrency: parse_or("RUNNER_CONCURRENCY", "64")?,
            mail,
            mail_from: var("MAIL_FROM")
                .unwrap_or_else(|| "DSA Visualized <no-reply@localhost>".into()),
            ai,
            ai_model: var("AI_MODEL"),
            policy: Policy {
                premium_tiers,
                runs_free_per_min: parse_or("RUNS_FREE_PER_MIN", "10")?,
                runs_pro_per_min: parse_or("RUNS_PRO_PER_MIN", "30")?,
                ai_free_daily: parse_or("AI_FREE_DAILY", "20")?,
                ai_pro_daily: parse_or("AI_PRO_DAILY", "300")?,
                require_verified_email: flag("REQUIRE_VERIFIED_EMAIL", false)?,
                signup_enabled: flag("SIGNUP_ENABLED", true)?,
                profiles_per_account: parse_or("PROFILES_PER_ACCOUNT", "5")?,
                draft_bytes: 64 * 1024,
            },
            admin_emails: var("ADMIN_EMAILS")
                .map(|s| {
                    s.split(',')
                        .map(|e| e.trim().to_lowercase())
                        .filter(|e| !e.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            github: oauth_client("OAUTH_GITHUB"),
            google: oauth_client("OAUTH_GOOGLE"),
            stripe,
            aws_region: var("AWS_REGION").or_else(|| var("AWS_DEFAULT_REGION")),
            log_format,
        })
    }
}

/// What `dsa-api migrate` needs, and nothing more: the one-off migration task
/// carries only database settings (no session secret, no public URL), so it
/// must not be held to the full server configuration's rules.
pub fn migration_env() -> Result<(PgConnectOptions, LogFormat)> {
    let prod = matches!(var("DSA_ENV").as_deref(), Some("production" | "prod"));
    let log = match var("LOG_FORMAT").as_deref() {
        Some("json") => LogFormat::Json,
        Some("pretty") => LogFormat::Pretty,
        _ if prod => LogFormat::Json,
        _ => LogFormat::Pretty,
    };
    Ok((db_options(prod)?, log))
}

/// A non-empty environment variable.
fn var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn parse_or<T: FromStr>(name: &str, default: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let raw = var(name).unwrap_or_else(|| default.to_string());
    raw.trim()
        .parse::<T>()
        .map_err(|e| anyhow!("{name}={raw:?}: {e}"))
}

fn parse_req<T: FromStr>(name: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let raw = var(name).ok_or_else(|| anyhow!("{name} is required"))?;
    raw.trim()
        .parse::<T>()
        .map_err(|e| anyhow!("{name}={raw:?}: {e}"))
}

fn parse_bool(name: &str, v: &str) -> Result<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("{name} must be true or false, got {v:?}"),
    }
}

fn flag(name: &str, default: bool) -> Result<bool> {
    match var(name) {
        Some(v) => parse_bool(name, &v),
        None => Ok(default),
    }
}

/// `scheme://host[:port]` with the default port dropped, lower-cased — the
/// exact form browsers send in the `Origin` header.
pub fn normalize_origin(raw: &str) -> Result<String> {
    let u = Url::parse(raw.trim()).with_context(|| format!("not a URL: {raw:?}"))?;
    let host = u
        .host_str()
        .ok_or_else(|| anyhow!("origin has no host: {raw:?}"))?
        .to_ascii_lowercase();
    Ok(match u.port() {
        Some(p) => format!("{}://{host}:{p}", u.scheme()),
        None => format!("{}://{host}", u.scheme()),
    })
}

/// Secrets arrive as base64, hex or (for development) plain text. Any of them
/// is fine as long as the result carries enough entropy.
fn decode_secret(s: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let s = s.trim();
    if s.len() >= 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(hex::decode(s)?);
    }
    for engine in [
        &base64::engine::general_purpose::STANDARD,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
    ] {
        if let Ok(b) = engine.decode(s) {
            if b.len() >= 32 {
                return Ok(b);
            }
        }
    }
    Ok(s.as_bytes().to_vec())
}

/// `DSA_CONTENT_DIR`, or the first conventional location that has content.
pub fn content_dir() -> PathBuf {
    if let Some(p) = var("DSA_CONTENT_DIR") {
        return PathBuf::from(p);
    }
    // The image bakes content in at /app/content; a checkout has it two
    // levels above `backend/`.
    for candidate in ["/app/content", "./content", "../content", "../../content"] {
        let p = PathBuf::from(candidate);
        if p.join("catalog.toml").is_file() {
            return p;
        }
    }
    PathBuf::from("../content")
}

/// `DATABASE_URL`, or the individual `DB_*` parts ECS injects from the RDS
/// secret. Building options from parts avoids URL-encoding a generated
/// password full of `/`, `@` and `%`.
fn db_options(prod: bool) -> Result<PgConnectOptions> {
    let sslmode = match var("DB_SSLMODE").as_deref() {
        Some(m) => PgSslMode::from_str(m).map_err(|e| anyhow!("DB_SSLMODE: {e}"))?,
        None if prod => PgSslMode::Require,
        None => PgSslMode::Prefer,
    };
    let opts = if let Some(url) = var("DATABASE_URL") {
        let mut o = PgConnectOptions::from_str(&url).context("DATABASE_URL")?;
        if var("DB_SSLMODE").is_some() || prod {
            o = o.ssl_mode(sslmode);
        }
        o
    } else {
        PgConnectOptions::new()
            .host(&var("DB_HOST").ok_or_else(|| anyhow!("DATABASE_URL or DB_HOST is required"))?)
            .port(parse_or("DB_PORT", "5432")?)
            .database(&var("DB_NAME").unwrap_or_else(|| "dsa".into()))
            .username(&var("DB_USER").unwrap_or_else(|| "dsa".into()))
            .password(&var("DB_PASSWORD").unwrap_or_default())
            .ssl_mode(sslmode)
    };
    Ok(opts.application_name("dsa-api"))
}

/// `REDIS_URL`, or `REDIS_HOST`/`REDIS_PORT`/`REDIS_TLS` plus an optional
/// `REDIS_AUTH_TOKEN` (ElastiCache AUTH arrives as its own secret so it never
/// has to be spliced into a URL by hand).
fn redis_url() -> Result<Option<String>> {
    let token = var("REDIS_AUTH_TOKEN");
    let base = match (var("REDIS_URL"), var("REDIS_HOST")) {
        (Some(url), _) => url,
        (None, Some(host)) => {
            let tls = flag("REDIS_TLS", true)?;
            let port: u16 = parse_or("REDIS_PORT", "6379")?;
            format!("{}://{host}:{port}", if tls { "rediss" } else { "redis" })
        }
        (None, None) => return Ok(None),
    };
    match token {
        None => Ok(Some(base)),
        Some(token) => {
            let mut u = Url::parse(&base).context("REDIS_URL")?;
            u.set_password(Some(&token))
                .map_err(|_| anyhow!("REDIS_URL cannot carry a password"))?;
            if u.username().is_empty() {
                // redis-rs treats an empty user with a password as AUTH <password>.
                let _ = u.set_username("");
            }
            Ok(Some(u.to_string()))
        }
    }
}

fn oauth_client(prefix: &str) -> Option<OAuthClient> {
    Some(OAuthClient {
        client_id: var(&format!("{prefix}_CLIENT_ID"))?,
        client_secret: var(&format!("{prefix}_CLIENT_SECRET"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_normalize_to_what_browsers_send() {
        assert_eq!(
            normalize_origin("https://Example.com/").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_origin("https://example.com:443/path").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_origin("http://localhost:5173").unwrap(),
            "http://localhost:5173"
        );
    }

    #[test]
    fn secrets_decode_from_hex_base64_or_text() {
        let hex = "ab".repeat(32);
        assert_eq!(decode_secret(&hex).unwrap().len(), 32);
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [7u8; 48]);
        assert_eq!(decode_secret(&b64).unwrap(), vec![7u8; 48]);
        assert_eq!(decode_secret("short").unwrap(), b"short".to_vec());
    }

    #[test]
    fn booleans_are_strict() {
        assert!(parse_bool("X", "yes").unwrap());
        assert!(!parse_bool("X", "0").unwrap());
        assert!(parse_bool("X", "maybe").is_err());
    }
}
