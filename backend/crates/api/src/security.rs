//! Cryptographic building blocks: random tokens, storage hashes, CSRF tokens,
//! signed cookies and password hashing.
//!
//! Two rules run through all of it:
//!
//! * **Secrets are stored hashed.** Session and email tokens are 256 random
//!   bits handed to the client once; the database only ever sees their SHA-256,
//!   so a leaked dump contains nothing that logs anyone in.
//! * **Comparisons are constant-time.** Anything checked against an
//!   attacker-supplied value goes through `subtle`.

use anyhow::{anyhow, Result};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// A fresh 256-bit token, URL-safe.
pub fn random_token() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    B64.encode(b)
}

/// What the database stores for a token.
pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

fn hmac(secret: &[u8], parts: &[&[u8]]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC takes any key length");
    for p in parts {
        mac.update(p);
        mac.update(&[0x1f]);
    }
    mac.finalize().into_bytes().to_vec()
}

/// The CSRF token for a session: an HMAC of the session id under the server
/// secret. It is unforgeable without the secret, useless for any other
/// session, and needs no storage — any API task can recompute it.
pub fn csrf_token(secret: &[u8], session_id: &uuid::Uuid) -> String {
    B64.encode(hmac(secret, &[b"csrf", session_id.as_bytes()]))
}

pub fn verify_csrf(secret: &[u8], session_id: &uuid::Uuid, presented: &str) -> bool {
    ct_eq(
        csrf_token(secret, session_id).as_bytes(),
        presented.trim().as_bytes(),
    )
}

/// `base64(payload).base64(hmac)` — a tamper-evident cookie value for short-
/// lived state such as an OAuth round trip.
pub fn sign(secret: &[u8], purpose: &str, payload: &[u8]) -> String {
    let body = B64.encode(payload);
    let sig = B64.encode(hmac(secret, &[purpose.as_bytes(), body.as_bytes()]));
    format!("{body}.{sig}")
}

pub fn unsign(secret: &[u8], purpose: &str, value: &str) -> Option<Vec<u8>> {
    let (body, sig) = value.split_once('.')?;
    let expected = B64.encode(hmac(secret, &[purpose.as_bytes(), body.as_bytes()]));
    if !ct_eq(expected.as_bytes(), sig.as_bytes()) {
        return None;
    }
    B64.decode(body).ok()
}

/// PKCE S256 challenge for a verifier.
pub fn pkce_challenge(verifier: &str) -> String {
    B64.encode(Sha256::digest(verifier.as_bytes()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Passwords
// ─────────────────────────────────────────────────────────────────────────────

pub const PASSWORD_MIN: usize = 10;
/// Argon2 is happy with long input, but an unbounded field is a CPU lever.
pub const PASSWORD_MAX: usize = 256;

/// Argon2id with the OWASP-recommended floor: 19 MiB, 2 passes, 1 lane —
/// ~20 ms on a Fargate vCPU, which is the right order for a login.
static ARGON: LazyLock<Argon2<'static>> = LazyLock::new(|| {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params"),
    )
});

/// A hash of a password nobody has, used to make "no such user" take as long
/// as "wrong password" so response time does not reveal which accounts exist.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("correct horse battery staple").expect("dummy hash"));

/// Human-readable problems with a candidate password, or `None` if it is fine.
pub fn password_problem(password: &str, email: &str) -> Option<String> {
    let n = password.chars().count();
    if n < PASSWORD_MIN {
        return Some(format!("Use at least {PASSWORD_MIN} characters."));
    }
    if n > PASSWORD_MAX {
        return Some(format!("Use at most {PASSWORD_MAX} characters."));
    }
    if password.trim().is_empty() {
        return Some("The password cannot be only spaces.".into());
    }
    let lower = password.to_lowercase();
    let local = email.split('@').next().unwrap_or("").to_lowercase();
    if lower == email.to_lowercase() || (local.len() >= 4 && lower.contains(&local)) {
        return Some("Don't build the password from your email address.".into());
    }
    let distinct: std::collections::BTreeSet<char> = password.chars().collect();
    if distinct.len() < 4 {
        return Some("That password is too repetitive.".into());
    }
    None
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    ARGON
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow!("argon2: {e}"))
}

/// Verify against a stored PHC string. `None` runs a dummy verification so
/// the timing matches a real one.
pub fn verify_password(password: &str, stored: Option<&str>) -> bool {
    let (hash, real) = match stored {
        Some(h) => (h, true),
        None => (DUMMY_HASH.as_str(), false),
    };
    let ok = PasswordHash::new(hash)
        .map(|parsed| ARGON.verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false);
    ok && real
}

/// CPU-bound hashing off the async executor.
pub async fn hash_password_async(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || hash_password(&password)).await?
}

pub async fn verify_password_async(password: String, stored: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || verify_password(&password, stored.as_deref()))
        .await
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Emails
// ─────────────────────────────────────────────────────────────────────────────

/// Lower-cased and trimmed: what uniqueness is enforced on.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Deliberately shallow: real validation is the verification email. This only
/// catches typos that could never be delivered.
pub fn email_problem(email: &str) -> Option<&'static str> {
    let e = email.trim();
    if e.len() > 254 {
        return Some("That email address is too long.");
    }
    let Some((local, domain)) = e.rsplit_once('@') else {
        return Some("That doesn't look like an email address.");
    };
    if local.is_empty()
        || domain.len() < 3
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || e.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Some("That doesn't look like an email address.");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_random_and_hash_stably() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
        assert_eq!(token_hash(&a), token_hash(&a));
        assert_eq!(token_hash(&a).len(), 32);
    }

    #[test]
    fn csrf_tokens_bind_to_the_session() {
        let secret = [9u8; 32];
        let s1 = uuid::Uuid::now_v7();
        let s2 = uuid::Uuid::now_v7();
        let t = csrf_token(&secret, &s1);
        assert!(verify_csrf(&secret, &s1, &t));
        assert!(!verify_csrf(&secret, &s2, &t), "another session's token");
        assert!(!verify_csrf(&[8u8; 32], &s1, &t), "another server secret");
        assert!(!verify_csrf(&secret, &s1, ""));
    }

    #[test]
    fn signed_values_detect_tampering() {
        let secret = [1u8; 32];
        let v = sign(&secret, "oauth", b"{\"state\":\"x\"}");
        assert_eq!(unsign(&secret, "oauth", &v).unwrap(), b"{\"state\":\"x\"}");
        assert!(unsign(&secret, "other-purpose", &v).is_none());
        let mut forged = v.clone();
        forged.replace_range(0..1, if v.starts_with('A') { "B" } else { "A" });
        assert!(unsign(&secret, "oauth", &forged).is_none());
    }

    #[test]
    fn passwords_hash_and_verify() {
        let h = hash_password("a perfectly fine passphrase").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(verify_password("a perfectly fine passphrase", Some(&h)));
        assert!(!verify_password("a perfectly fine passphrasE", Some(&h)));
        assert!(
            !verify_password("anything", None),
            "no account never verifies"
        );
    }

    #[test]
    fn weak_passwords_are_explained() {
        assert!(password_problem("short", "a@b.co").is_some());
        assert!(password_problem("aaaaaaaaaaaa", "a@b.co").is_some());
        assert!(password_problem("samantha-2024!", "samantha@x.com").is_some());
        assert!(password_problem("tangerine lantern 7", "sam@x.com").is_none());
    }

    #[test]
    fn email_checks_catch_only_obvious_typos() {
        assert!(email_problem("sam@example.com").is_none());
        assert!(email_problem("sam@localhost").is_some());
        assert!(email_problem("sam example.com").is_some());
        assert!(email_problem("@example.com").is_some());
        assert_eq!(normalize_email("  Sam@Example.COM "), "sam@example.com");
    }

    #[test]
    fn pkce_matches_the_rfc_example() {
        // RFC 7636, appendix B.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}
