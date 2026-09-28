//! End-to-end tests of the HTTP API against a real PostgreSQL.
//!
//! Each test gets its own freshly migrated database (`#[sqlx::test]`, from
//! `DATABASE_URL`) and drives the real router — middleware, extractors, SQL
//! and all. External services are replaced by small local fakes: a runner
//! that "compiles" programs, an Ollama that streams, and a mailer that keeps
//! the links it would have sent.
//!
//! ```text
//! DATABASE_URL=postgres://dsa:dsa@localhost:55432/dsa cargo test -p dsa-api --test api
//! ```

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::Router;
use dsa_api::config::{AiProviderConfig, Config, Env, LogFormat, MailMode, Policy, RunnerMode};
use dsa_api::mail::{Email, Mailer};
use dsa_protocol::{
    CaseOutcome, CaseStatus, CompileOutcome, ExecuteRequest, ExecuteResponse, PROTOCOL_VERSION,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower::ServiceExt;

const ORIGIN: &str = "http://localhost:5173";

// ─────────────────────────────────────────────────────────────────────────────
// Harness
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Outbox(Arc<Mutex<Vec<Email>>>);

#[async_trait]
impl Mailer for Outbox {
    async fn send(&self, email: Email) -> anyhow::Result<()> {
        self.0.lock().unwrap().push(email);
        Ok(())
    }
}

impl Outbox {
    /// The `token=` from the last email to `to`, waiting briefly because
    /// emails are sent from a spawned task.
    async fn token_for(&self, to: &str) -> String {
        for _ in 0..50 {
            if let Some(e) = self.0.lock().unwrap().iter().rev().find(|e| e.to == to) {
                let i = e.text.find("token=").expect("link has a token") + 6;
                return e.text[i..].split_whitespace().next().unwrap().to_string();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("no email to {to}");
    }
}

fn config(runner: RunnerMode, ai: AiProviderConfig, tweak: impl FnOnce(&mut Config)) -> Config {
    let mut c = Config {
        env: Env::Development,
        bind: "127.0.0.1:0".parse().unwrap(),
        metrics_bind: None,
        public_url: ORIGIN.parse().unwrap(),
        allowed_origins: vec![ORIGIN.into()],
        content_dir: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
        db: sqlx::postgres::PgConnectOptions::new(),
        db_max_connections: 5,
        redis_url: None,
        session_secret: vec![7u8; 32],
        session_ttl: Duration::from_secs(86_400),
        cookie_secure: false,
        run_migrations: false,
        trusted_proxy_hops: 0,
        origin_verify_secret: None,
        runner,
        runner_concurrency: 8,
        mail: MailMode::Log,
        mail_from: "test <t@test.local>".into(),
        ai,
        ai_model: Some("test-model".into()),
        policy: Policy {
            premium_tiers: vec![],
            runs_free_per_min: 100,
            runs_pro_per_min: 100,
            ai_free_daily: 5,
            ai_pro_daily: 50,
            require_verified_email: false,
            signup_enabled: true,
            profiles_per_account: 3,
            draft_bytes: 64 * 1024,
        },
        admin_emails: vec![],
        github: None,
        google: None,
        stripe: None,
        aws_region: None,
        log_format: LogFormat::Pretty,
    };
    tweak(&mut c);
    c
}

async fn app(pool: PgPool, cfg: Config) -> (Router, Outbox) {
    let outbox = Outbox::default();
    let state = dsa_api::build_state_with(cfg, pool, Some(Box::new(outbox.clone())))
        .await
        .expect("state builds");
    (dsa_api::app::build(state), outbox)
}

async fn plain(pool: PgPool) -> (Router, Outbox) {
    app(
        pool,
        config(RunnerMode::Disabled, AiProviderConfig::None, |_| {}),
    )
    .await
}

/// A browser-like client: keeps the session cookie and CSRF token, sends an
/// `Origin` on writes the way browsers do.
struct Client {
    app: Router,
    cookie: Option<String>,
    csrf: Option<String>,
}

struct Resp {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
    raw: bytes::Bytes,
}

impl Client {
    fn new(app: &Router) -> Self {
        Self {
            app: app.clone(),
            cookie: None,
            csrf: None,
        }
    }

    async fn req(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        extra: &[(&str, &str)],
    ) -> Resp {
        let mut b = Request::builder()
            .method(method.clone())
            .uri(format!("/api/v1{path}"));
        let own_origin = !extra.iter().any(|(k, _)| k.eq_ignore_ascii_case("origin"));
        if method != Method::GET {
            if own_origin {
                b = b.header(header::ORIGIN, ORIGIN);
            }
            if let Some(t) = &self.csrf {
                b = b.header("x-csrf-token", t);
            }
        }
        if let Some(c) = &self.cookie {
            b = b.header(header::COOKIE, c);
        }
        for (k, v) in extra {
            b = b.header(*k, *v);
        }
        let req = match body {
            Some(v) => b
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let raw = res.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&raw).unwrap_or(Value::Null);
        // Adopt a new session exactly as a browser would.
        if let Some(set) = headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
        {
            if let Some(pair) = set.split(';').next() {
                if pair.starts_with("dsa_session=") {
                    self.cookie = if pair == "dsa_session=" {
                        None
                    } else {
                        Some(pair.to_string())
                    };
                }
            }
        }
        if let Some(t) = body.get("csrf_token").and_then(|v| v.as_str()) {
            self.csrf = Some(t.to_string());
        }
        Resp {
            status,
            headers,
            body,
            raw,
        }
    }

    async fn get(&mut self, path: &str) -> Resp {
        self.req(Method::GET, path, None, &[]).await
    }
    async fn post(&mut self, path: &str, body: Value) -> Resp {
        self.req(Method::POST, path, Some(body), &[]).await
    }
    async fn put(&mut self, path: &str, body: Value) -> Resp {
        self.req(Method::PUT, path, Some(body), &[]).await
    }
    async fn patch(&mut self, path: &str, body: Value) -> Resp {
        self.req(Method::PATCH, path, Some(body), &[]).await
    }
    async fn delete(&mut self, path: &str, body: Option<Value>) -> Resp {
        self.req(Method::DELETE, path, body, &[]).await
    }

    async fn signup(&mut self, email: &str) -> Value {
        let r = self
            .post(
                "/auth/signup",
                json!({ "email": email, "password": "tangerine lantern 7", "display_name": "Sam" }),
            )
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.body
    }
}

fn pid(session: &Value) -> String {
    session["profiles"][0]["id"].as_str().unwrap().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Content and traces
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn content_is_public_and_revalidates(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);

    let cat = c.get("/content/catalog").await;
    assert_eq!(cat.status, StatusCode::OK);
    assert_eq!(cat.body["total"], 287);
    assert!(cat.headers[header::CACHE_CONTROL]
        .to_str()
        .unwrap()
        .starts_with("public"));
    let etag = cat.headers[header::ETAG].to_str().unwrap().to_string();

    let again = c
        .req(
            Method::GET,
            "/content/catalog",
            None,
            &[("if-none-match", &etag)],
        )
        .await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);

    let p = c.get("/content/problems/two-sum").await;
    assert_eq!(p.status, StatusCode::OK);
    assert_eq!(p.body["sources"].as_array().unwrap().len(), 3);
    assert_eq!(p.body["locked"], false);

    let t = c.get("/content/problems/two-sum/trace").await;
    assert_eq!(t.status, StatusCode::OK);
    assert!(t.body["trace"]["steps"].as_array().unwrap().len() > 5);

    assert_eq!(
        c.get("/content/problems/no-such-problem").await.status,
        StatusCode::NOT_FOUND
    );
    let g = c.get("/content/guide").await;
    assert!(g.body["topics"].as_array().unwrap().len() >= 20);
    // Everything that is not public content defaults to no-store.
    let m = c.get("/meta").await;
    assert_eq!(m.headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(m.body["features"]["runner"], false);
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn custom_traces_parse_and_validate_like_the_desktop(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);

    let ok = c
        .post(
            "/problems/two-sum/trace",
            json!({ "fields": { "nums": "2 7 11 15", "target": "9" } }),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.body);
    assert_eq!(ok.body["input"]["nums"], json!([2, 7, 11, 15]));
    assert_eq!(ok.body["trace"]["result"], "[0, 1]");

    let bad = c
        .post(
            "/problems/two-sum/trace",
            json!({ "fields": { "nums": "2 x", "target": "nine" } }),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors = bad.body["error"]["details"]["errors"].as_array().unwrap();
    assert_eq!(
        errors.len(),
        2,
        "every bad field is reported at once: {errors:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn signup_session_logout_and_csrf(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let s = c.signup("sam@example.com").await;
    assert_eq!(s["user"]["email"], "sam@example.com");
    assert_eq!(s["user"]["email_verified"], false);
    assert_eq!(
        s["profiles"].as_array().unwrap().len(),
        1,
        "a first profile comes with the account"
    );
    assert!(c.cookie.is_some());

    assert_eq!(c.get("/auth/session").await.status, StatusCode::OK);

    // A write without the CSRF token is refused even with a valid cookie.
    let token = c.csrf.take();
    let r = c.post("/auth/logout", json!({})).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.body["error"]["code"], "csrf");
    c.csrf = token;

    assert_eq!(
        c.post("/auth/logout", json!({})).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.get("/auth/session").await.status,
        StatusCode::UNAUTHORIZED
    );

    let mut d = Client::new(&app);
    let dup = d
        .post(
            "/auth/signup",
            json!({ "email": "SAM@example.com", "password": "another fine one 9", "display_name": "Sam 2" }),
        )
        .await;
    assert_eq!(
        dup.status,
        StatusCode::CONFLICT,
        "emails are unique case-insensitively"
    );

    let wrong = d
        .post(
            "/auth/login",
            json!({ "email": "sam@example.com", "password": "nope nope nope" }),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    let ghost = d
        .post(
            "/auth/login",
            json!({ "email": "ghost@example.com", "password": "nope nope nope" }),
        )
        .await;
    assert_eq!(
        ghost.body["error"]["message"], wrong.body["error"]["message"],
        "no account enumeration"
    );
    let ok = d
        .post(
            "/auth/login",
            json!({ "email": "Sam@Example.com", "password": "tangerine lantern 7" }),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn weak_passwords_and_bad_emails_are_explained_per_field(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let r = c
        .post(
            "/auth/signup",
            json!({ "email": "not-an-email", "password": "short", "display_name": "" }),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let fields = &r.body["error"]["details"]["fields"];
    assert!(fields["email"].is_string());
    assert!(fields["password"].is_string());
    assert!(fields["display_name"].is_string());
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn cross_origin_writes_are_refused(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let r = c
        .req(
            Method::POST,
            "/auth/login",
            Some(json!({ "email": "a@b.co", "password": "x" })),
            &[("origin", "https://evil.example")],
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);

    // A second, allowed Origin next to a hostile one does not launder it.
    let both = c
        .req(
            Method::POST,
            "/auth/login",
            Some(json!({ "email": "a@b.co", "password": "x" })),
            &[("origin", ORIGIN), ("origin", "https://evil.example")],
        )
        .await;
    assert_eq!(both.status, StatusCode::FORBIDDEN, "{}", both.body);
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn ten_misses_lock_the_account(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    c.signup("lock@example.com").await;
    let mut d = Client::new(&app);
    let mut seen = Vec::new();
    for _ in 0..10 {
        seen.push(
            d.post(
                "/auth/login",
                json!({ "email": "lock@example.com", "password": "definitely wrong" }),
            )
            .await
            .status
            .as_u16(),
        );
    }
    assert_eq!(
        seen,
        [[401u16; 9].as_slice(), &[423]].concat(),
        "nine misses, then the lock"
    );
    // Even the right password waits it out. The per-address rate limit (ten
    // tries a quarter hour) sits in front of the lock, so either may answer.
    let r = d
        .post(
            "/auth/login",
            json!({ "email": "lock@example.com", "password": "tangerine lantern 7" }),
        )
        .await;
    assert!(
        r.status == StatusCode::LOCKED || r.status == StatusCode::TOO_MANY_REQUESTS,
        "got {}",
        r.status
    );
    assert!(
        r.body["error"]["details"]["retry_after_secs"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(d.cookie.is_none(), "no session was issued");
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn password_reset_signs_out_everywhere(pool: PgPool) {
    let (app, outbox) = plain(pool).await;
    let mut c = Client::new(&app);
    c.signup("reset@example.com").await;

    let mut anon = Client::new(&app);
    assert_eq!(
        anon.post(
            "/auth/password/forgot",
            json!({ "email": "reset@example.com" })
        )
        .await
        .status,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        anon.post(
            "/auth/password/forgot",
            json!({ "email": "nobody@example.com" })
        )
        .await
        .status,
        StatusCode::ACCEPTED
    );
    let token = outbox.token_for("reset@example.com").await;

    let weak = anon
        .post(
            "/auth/password/reset",
            json!({ "token": token, "password": "short" }),
        )
        .await;
    assert_eq!(weak.status, StatusCode::UNPROCESSABLE_ENTITY);
    // The weak attempt did not spend the link.
    let ok = anon
        .post(
            "/auth/password/reset",
            json!({ "token": token, "password": "brand new passphrase 3" }),
        )
        .await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT, "{}", ok.body);
    let again = anon
        .post(
            "/auth/password/reset",
            json!({ "token": token, "password": "brand new passphrase 4" }),
        )
        .await;
    assert_eq!(
        again.status,
        StatusCode::BAD_REQUEST,
        "links are single-use"
    );

    assert_eq!(
        c.get("/auth/session").await.status,
        StatusCode::UNAUTHORIZED,
        "old sessions are revoked"
    );
    let login = anon
        .post(
            "/auth/login",
            json!({ "email": "reset@example.com", "password": "brand new passphrase 3" }),
        )
        .await;
    assert_eq!(login.status, StatusCode::OK);
    assert_eq!(
        login.body["user"]["email_verified"], true,
        "the reset email proved the address"
    );
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn admin_rights_follow_a_verified_address_only(pool: PgPool) {
    let cfg = config(RunnerMode::Disabled, AiProviderConfig::None, |c| {
        c.admin_emails = vec!["boss@example.com".into()]
    });
    let (app, outbox) = app(pool, cfg).await;
    let mut c = Client::new(&app);
    let s = c.signup("boss@example.com").await;
    assert_eq!(
        s["user"]["role"], "user",
        "not before the address is verified"
    );
    assert_eq!(c.get("/admin/overview").await.status, StatusCode::NOT_FOUND);

    let token = outbox.token_for("boss@example.com").await;
    assert_eq!(
        c.post("/auth/verify-email", json!({ "token": token }))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let me = c.get("/me").await;
    assert_eq!(me.body["role"], "admin");
    let o = c.get("/admin/overview").await;
    assert_eq!(o.status, StatusCode::OK);
    assert_eq!(o.body["users"], 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiles, progress, playlists, drafts, settings, stats
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn progress_follows_the_desktop_rules(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let p = pid(&c.signup("prog@example.com").await);

    let s = c
        .put(
            &format!("/profiles/{p}/progress/two-sum/status"),
            json!({ "status": "solved" }),
        )
        .await;
    assert_eq!(s.status, StatusCode::OK, "{}", s.body);
    assert_eq!(s.body["status"], "solved");
    assert!(s.body["solved_at"].is_string());
    c.put(
        &format!("/profiles/{p}/progress/3sum/favourite"),
        json!({ "favourite": true }),
    )
    .await;

    let snap = c.get(&format!("/profiles/{p}/progress")).await;
    assert_eq!(
        snap.body["stats"],
        json!({ "solved": 1, "attempted": 0, "favourites": 1 })
    );
    assert_eq!(
        snap.body["entries"]["3sum"]["status"], "todo",
        "a favourite alone is still to do"
    );

    // Unknown problems cannot be written.
    let bad = c
        .put(
            &format!("/profiles/{p}/progress/not-a-problem/status"),
            json!({ "status": "solved" }),
        )
        .await;
    assert_eq!(bad.status, StatusCode::NOT_FOUND);

    let stats = c.get(&format!("/profiles/{p}/stats")).await;
    assert_eq!(stats.status, StatusCode::OK);
    assert_eq!(stats.body["totals"]["solved"], 1);
    assert_eq!(stats.body["streak"]["current"], 1);
    assert_eq!(stats.body["streak"]["active_today"], true);
    let t50 = &stats.body["by_tier"][0];
    assert_eq!(
        (
            t50["tier"].as_str(),
            t50["solved"].as_u64(),
            t50["total"].as_u64()
        ),
        (Some("50"), Some(1), Some(50))
    );
    assert_eq!(stats.body["by_category"].as_array().unwrap().len(), 22);

    // Un-ticking is deliberate and allowed.
    let back = c
        .put(
            &format!("/profiles/{p}/progress/two-sum/status"),
            json!({ "status": "todo" }),
        )
        .await;
    assert_eq!(back.body["status"], "todo");
    assert!(back.body["solved_at"].is_null());
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn playlists_drafts_and_settings(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let p = pid(&c.signup("lists@example.com").await);

    let pl = c
        .post(
            &format!("/profiles/{p}/playlists"),
            json!({ "name": "hard ones", "slugs": ["trapping-rain-water"] }),
        )
        .await;
    assert_eq!(pl.status, StatusCode::CREATED, "{}", pl.body);
    let id = pl.body["id"].as_str().unwrap().to_string();
    assert_eq!(
        c.post(
            &format!("/profiles/{p}/playlists"),
            json!({ "name": "Hard Ones" })
        )
        .await
        .status,
        StatusCode::CONFLICT
    );

    assert_eq!(
        c.put(
            &format!("/profiles/{p}/playlists/{id}/items/3sum"),
            json!({})
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        c.put(
            &format!("/profiles/{p}/playlists/{id}/items/3sum"),
            json!({})
        )
        .await
        .status,
        StatusCode::NO_CONTENT,
        "idempotent"
    );
    let lists = c.get(&format!("/profiles/{p}/playlists")).await;
    assert_eq!(
        lists.body[0]["slugs"],
        json!(["trapping-rain-water", "3sum"])
    );
    c.delete(
        &format!("/profiles/{p}/playlists/{id}/items/trapping-rain-water"),
        None,
    )
    .await;
    let renamed = c
        .patch(
            &format!("/profiles/{p}/playlists/{id}"),
            json!({ "name": "week 1" }),
        )
        .await;
    assert_eq!(renamed.body["name"], "week 1");
    assert_eq!(renamed.body["slugs"], json!(["3sum"]));

    assert_eq!(
        c.put(
            &format!("/profiles/{p}/drafts/two-sum/go"),
            json!({ "code": "func twoSum() {}" })
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let d = c.get(&format!("/profiles/{p}/drafts/two-sum")).await;
    assert_eq!(d.body["go"]["code"], "func twoSum() {}");
    c.delete(&format!("/profiles/{p}/drafts/two-sum/go"), None)
        .await;
    assert_eq!(
        c.get(&format!("/profiles/{p}/drafts/two-sum")).await.body,
        json!({})
    );

    let s = c
        .put(
            &format!("/profiles/{p}/settings"),
            json!({ "lang": "cpp", "speed": 2.0 }),
        )
        .await;
    assert_eq!(s.body["lang"], "cpp");
    let s = c
        .put(&format!("/profiles/{p}/settings"), json!({ "tier": "250" }))
        .await;
    assert_eq!(
        s.body,
        json!({ "lang": "cpp", "speed": 2.0, "tier": "250" }),
        "merge, not replace"
    );
    assert_eq!(
        c.put(&format!("/profiles/{p}/settings"), json!({ "nope": 1 }))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn profiles_belong_to_their_account(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut a = Client::new(&app);
    let pa = pid(&a.signup("a@example.com").await);
    let mut b = Client::new(&app);
    b.signup("b@example.com").await;

    assert_eq!(
        b.get(&format!("/profiles/{pa}/progress")).await.status,
        StatusCode::NOT_FOUND,
        "404, never 403"
    );

    let second = a
        .post(
            "/profiles",
            json!({ "name": "revision", "avatar": "🎯", "color": "#22d3ee" }),
        )
        .await;
    assert_eq!(second.status, StatusCode::CREATED, "{}", second.body);
    assert_eq!(
        a.post(
            "/profiles",
            json!({ "name": "Revision", "avatar": "🎯", "color": "#22d3ee" })
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        a.post(
            "/profiles",
            json!({ "name": "x", "avatar": "💀", "color": "#22d3ee" })
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    a.post(
        "/profiles",
        json!({ "name": "third", "avatar": "⚡", "color": "#34d399" }),
    )
    .await;
    let over = a
        .post(
            "/profiles",
            json!({ "name": "fourth", "avatar": "⚡", "color": "#34d399" }),
        )
        .await;
    assert_eq!(
        over.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "the per-account limit holds"
    );

    let all = a.get("/profiles").await;
    let ids: Vec<String> = all
        .body
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    for id in &ids[1..] {
        assert_eq!(
            a.delete(&format!("/profiles/{id}"), None).await.status,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(
        a.delete(&format!("/profiles/{}", ids[0]), None)
            .await
            .status,
        StatusCode::CONFLICT,
        "the last one stays"
    );
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn desktop_imports_merge_without_downgrading(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut c = Client::new(&app);
    let p = pid(&c.signup("import@example.com").await);
    c.put(
        &format!("/profiles/{p}/progress/two-sum/status"),
        json!({ "status": "solved" }),
    )
    .await;
    let r = c
        .post(
            &format!("/profiles/{p}/import"),
            json!({
                "entries": {
                    "two-sum": { "status": "attempted", "favourite": true, "attempts": 4 },
                    "3sum": { "status": "attempted", "favourite": false, "attempts": 2 },
                    "renamed-long-ago": { "status": "solved", "favourite": false, "attempts": 1 }
                },
                "playlists": [ { "name": "week 1", "slugs": ["two-sum", "3sum", "gone"] } ]
            }),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body, json!({ "progress_rows": 2, "playlists": 1 }));
    let snap = c.get(&format!("/profiles/{p}/progress")).await;
    assert_eq!(
        snap.body["entries"]["two-sum"]["status"], "solved",
        "never downgraded"
    );
    assert_eq!(snap.body["entries"]["two-sum"]["favourite"], true);
    assert_eq!(snap.body["entries"]["two-sum"]["attempts"], 4);
    let lists = c.get(&format!("/profiles/{p}/playlists")).await;
    assert_eq!(lists.body[0]["slugs"], json!(["two-sum", "3sum"]));
}

// ─────────────────────────────────────────────────────────────────────────────
// Runs, against a fake runner
// ─────────────────────────────────────────────────────────────────────────────

/// Pretends to compile: `SYNTAX` in the source fails the build; `CORRECT`
/// makes the "program" solve Two Sum from its stdin; anything else prints 0 0.
async fn fake_runner() -> url::Url {
    async fn execute(axum::Json(req): axum::Json<ExecuteRequest>) -> axum::Json<ExecuteResponse> {
        let build_ok = !req.source.contains("SYNTAX");
        let cases = if !build_ok {
            vec![]
        } else {
            req.cases
                .iter()
                .map(|c| {
                    let out = if req.source.contains("CORRECT") {
                        let mut lines = c.stdin.lines();
                        let nums: Vec<i64> = lines
                            .next()
                            .unwrap_or("")
                            .split_whitespace()
                            .map(|x| x.parse().unwrap())
                            .collect();
                        let target: i64 = lines.next().unwrap_or("0").trim().parse().unwrap();
                        let mut ans = String::from("none");
                        'outer: for i in 0..nums.len() {
                            for j in i + 1..nums.len() {
                                if nums[i] + nums[j] == target {
                                    ans = format!("{i} {j}");
                                    break 'outer;
                                }
                            }
                        }
                        ans + "\n"
                    } else {
                        "0 0\n".into()
                    };
                    CaseOutcome {
                        id: c.id.clone(),
                        status: CaseStatus::Ok,
                        stdout: out,
                        stderr: String::new(),
                        exit_code: Some(0),
                        signal: None,
                        duration_ms: 1,
                        stdout_truncated: false,
                        stderr_truncated: false,
                    }
                })
                .collect()
        };
        axum::Json(ExecuteResponse {
            protocol: PROTOCOL_VERSION,
            job_id: req.job_id,
            language: req.language,
            compile: Some(CompileOutcome {
                ok: build_ok,
                output: if build_ok {
                    String::new()
                } else {
                    "main.go:3:1: syntax error".into()
                },
                timed_out: false,
                truncated: false,
                duration_ms: 5,
            }),
            cases,
            error: None,
            runner_version: "fake".into(),
            duration_ms: 7,
        })
    }
    let app = Router::new().route("/v1/execute", axum::routing::post(execute));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/").parse().unwrap()
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn runs_record_attempts_and_a_green_sweep_solves(pool: PgPool) {
    let url = fake_runner().await;
    let cfg = config(
        RunnerMode::Http { url, token: None },
        AiProviderConfig::None,
        |_| {},
    );
    let (app, _) = app(pool, cfg).await;
    let mut c = Client::new(&app);
    let p = pid(&c.signup("runs@example.com").await);
    let path = format!("/profiles/{p}/runs");

    let wrong = c.post(&path, json!({ "slug": "two-sum", "lang": "go", "code": "func twoSum() {}", "mode": "solution", "kind": "test" })).await;
    assert_eq!(wrong.status, StatusCode::OK, "{}", wrong.body);
    assert_eq!(wrong.body["status"], "failed");
    assert_eq!(wrong.body["tests"][0]["status"], "fail");
    assert_eq!(wrong.body["tests"][0]["expected"], "2 4");
    assert_eq!(wrong.body["progress"]["status"], "attempted");
    assert_eq!(wrong.body["progress"]["attempts"], 1);

    let build = c.post(&path, json!({ "slug": "two-sum", "lang": "go", "code": "SYNTAX", "mode": "program", "kind": "test" })).await;
    assert_eq!(build.body["status"], "compile_error");
    assert!(build.body["tests"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("syntax error"));

    let run = c.post(&path, json!({ "slug": "two-sum", "lang": "go", "code": "CORRECT", "mode": "program", "kind": "run", "stdin": "3 3\n6\n" })).await;
    assert_eq!(run.body["status"], "ok");
    assert_eq!(run.body["stdout"], "0 1\n");

    let green = c.post(&path, json!({ "slug": "two-sum", "lang": "go", "code": "CORRECT", "mode": "program", "kind": "test" })).await;
    assert_eq!(green.body["status"], "passed", "{}", green.body);
    assert_eq!(green.body["passed"], green.body["total"]);
    assert_eq!(green.body["progress"]["status"], "solved");
    assert_eq!(green.body["progress"]["attempts"], 4);

    // A later failing run never un-solves it.
    let after = c.post(&path, json!({ "slug": "two-sum", "lang": "go", "code": "x", "mode": "program", "kind": "test" })).await;
    assert_eq!(after.body["progress"]["status"], "solved");

    let page = c.get(&format!("/profiles/{p}/submissions?limit=2")).await;
    assert_eq!(page.body["items"].as_array().unwrap().len(), 2);
    let cursor = page.body["next_cursor"].as_str().unwrap().to_string();
    let rest = c
        .get(&format!(
            "/profiles/{p}/submissions?limit=10&cursor={cursor}"
        ))
        .await;
    assert_eq!(rest.body["items"].as_array().unwrap().len(), 3);
    assert!(rest.body["next_cursor"].is_null());

    let id = green.body["id"].as_str().unwrap();
    let one = c.get(&format!("/profiles/{p}/submissions/{id}")).await;
    assert_eq!(one.body["code"], "CORRECT");
    assert_eq!(one.body["result"]["status"], "passed");
    assert_eq!(one.body["result"]["progress"]["status"], "solved");

    let stats = c.get(&format!("/profiles/{p}/stats")).await;
    assert_eq!(stats.body["totals"]["submissions"], 5);
    assert_eq!(stats.body["activity"][0]["runs"], 5);
    assert_eq!(stats.body["activity"][0]["solved"], 1);
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn guests_cannot_run_code(pool: PgPool) {
    let url = fake_runner().await;
    let cfg = config(
        RunnerMode::Http { url, token: None },
        AiProviderConfig::None,
        |_| {},
    );
    let (app, _) = app(pool, cfg).await;
    let mut c = Client::new(&app);
    let s = c.signup("owner@example.com").await;
    let p = pid(&s);
    let mut guest = Client::new(&app);
    let r = guest.post(&format!("/profiles/{p}/runs"), json!({ "slug": "two-sum", "lang": "go", "code": "x", "mode": "program", "kind": "run" })).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

// ─────────────────────────────────────────────────────────────────────────────
// AI, against a fake Ollama
// ─────────────────────────────────────────────────────────────────────────────

async fn fake_ollama() -> url::Url {
    async fn chat(axum::Json(body): axum::Json<Value>) -> impl axum::response::IntoResponse {
        // The system prompt must be the desktop's, filled for this problem.
        let system = body["messages"][0]["content"].as_str().unwrap_or("");
        assert!(system.contains("Two Sum"), "{system}");
        let lines = [
            json!({ "message": { "role": "assistant", "content": "", "thinking": "consider a map" }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "What would give you " }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "O(1) lookups?" }, "done": false }),
            json!({ "message": { "content": "" }, "done": true, "prompt_eval_count": 120, "eval_count": 9 }),
        ];
        lines.iter().map(|l| format!("{l}\n")).collect::<String>()
    }
    let app = Router::new().route("/api/chat", axum::routing::post(chat));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}").parse().unwrap()
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn ai_chat_streams_events_and_meters_the_quota(pool: PgPool) {
    let url = fake_ollama().await;
    let cfg = config(
        RunnerMode::Disabled,
        AiProviderConfig::Ollama { url },
        |c| c.policy.ai_free_daily = 2,
    );
    let (app, _) = app(pool, cfg).await;
    let mut c = Client::new(&app);
    c.signup("ai@example.com").await;

    let body = json!({
        "slug": "two-sum", "lang": "go", "mode": "interview",
        "messages": [ { "role": "user", "content": "am I on the right track?" } ]
    });
    let r = c.post("/ai/chat", body.clone()).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.headers[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
    let text = String::from_utf8(r.raw.to_vec()).unwrap();
    assert!(text.contains("event: token"), "{text}");
    assert!(text.contains(r#""channel":"thinking""#));
    assert!(text.contains("O(1) lookups?"));
    assert!(text.contains("event: done"));
    assert!(text.contains(r#""remaining_today":1"#), "{text}");

    let status = c.get("/ai/status").await;
    assert_eq!(status.body["used_today"], 1);

    assert_eq!(
        c.post("/ai/chat", body.clone()).await.status,
        StatusCode::OK
    );
    let over = c.post("/ai/chat", body).await;
    assert_eq!(over.status, StatusCode::PAYMENT_REQUIRED, "{}", over.body);

    let fix_without_code = c
        .post(
            "/ai/chat",
            json!({ "slug": "two-sum", "lang": "go", "mode": "fix", "messages": [] }),
        )
        .await;
    assert_eq!(fix_without_code.status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ─────────────────────────────────────────────────────────────────────────────
// Account
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn deleting_the_account_removes_everything(pool: PgPool) {
    let (app, _) = plain(pool.clone()).await;
    let mut c = Client::new(&app);
    let p = pid(&c.signup("bye@example.com").await);
    c.put(
        &format!("/profiles/{p}/progress/two-sum/status"),
        json!({ "status": "solved" }),
    )
    .await;

    let wrong = c
        .delete(
            "/me",
            Some(
                json!({ "confirm_email": "other@example.com", "password": "tangerine lantern 7" }),
            ),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
    let export = c.get("/me/export").await;
    assert_eq!(export.status, StatusCode::OK);
    assert_eq!(
        export.body["profiles"][0]["progress"]["two-sum"]["status"],
        "solved"
    );

    let ok = c
        .delete(
            "/me",
            Some(json!({ "confirm_email": "BYE@example.com", "password": "tangerine lantern 7" })),
        )
        .await;
    assert_eq!(ok.status, StatusCode::NO_CONTENT, "{}", ok.body);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM progress")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0, "cascaded");
    let mut d = Client::new(&app);
    assert_eq!(
        d.post(
            "/auth/login",
            json!({ "email": "bye@example.com", "password": "tangerine lantern 7" })
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrator = "dsa_api::db::MIGRATOR")]
async fn revoking_a_session_ends_it_on_that_device(pool: PgPool) {
    let (app, _) = plain(pool).await;
    let mut laptop = Client::new(&app);
    laptop.signup("two@example.com").await;
    let mut phone = Client::new(&app);
    phone
        .post(
            "/auth/login",
            json!({ "email": "two@example.com", "password": "tangerine lantern 7" }),
        )
        .await;

    let list = laptop.get("/auth/sessions").await;
    assert_eq!(list.body.as_array().unwrap().len(), 2);
    assert_eq!(
        laptop
            .post("/auth/sessions/revoke-others", json!({}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        phone.get("/auth/session").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(laptop.get("/auth/session").await.status, StatusCode::OK);
}
