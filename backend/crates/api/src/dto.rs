//! Wire types: the Rust side of `web/src/api/types.ts`.
//!
//! Every struct here serializes to exactly the shape declared there, field for
//! field. `Option` fields that the TS marks `T | null` serialize as `null`;
//! those it marks `key?: T` are skipped when empty. When one side changes, the
//! other changes in the same commit.

use dsa_core::model::Trace;
use dsa_core::problem::{Difficulty, InputField, InputMap, Tier};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;
use uuid::Uuid;

/// RFC 3339 on the wire.
pub type Ts = OffsetDateTime;

mod rfc3339 {
    pub use time::serde::rfc3339::{option, serialize};
}

// ─────────────────────────────────────────────────────────────────────────────
// Meta
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PlanInfo {
    pub id: &'static str,
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_monthly: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_yearly: Option<f64>,
    pub features: Vec<String>,
}

#[derive(Serialize)]
pub struct OAuthFlags {
    pub github: bool,
    pub google: bool,
}

#[derive(Serialize)]
pub struct AiFlags {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
}

#[derive(Serialize)]
pub struct Features {
    pub signup: bool,
    pub email_verification_required: bool,
    pub oauth: OAuthFlags,
    pub billing: bool,
    pub ai: AiFlags,
    pub runner: bool,
}

#[derive(Serialize)]
pub struct MetaLimits {
    pub profiles_per_account: usize,
    pub draft_bytes: usize,
}

#[derive(Serialize)]
pub struct Meta {
    pub version: &'static str,
    pub content_version: String,
    pub features: Features,
    pub plans: Vec<PlanInfo>,
    pub limits: MetaLimits,
    pub avatars: Vec<&'static str>,
    pub colors: Vec<&'static str>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth & account
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub role: String,
    pub plan: String,
    #[serde(with = "rfc3339::option")]
    pub plan_renews_at: Option<Ts>,
    pub timezone: String,
    pub has_password: bool,
    pub oauth_providers: Vec<String>,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
}

#[derive(Serialize, Clone)]
pub struct Entitlements {
    pub plan: String,
    pub premium_content: bool,
    pub ai_daily_limit: u32,
    pub ai_used_today: u32,
    pub runs_per_minute: u32,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub user: User,
    pub csrf_token: String,
    pub entitlements: Entitlements,
    pub profiles: Vec<Profile>,
}

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Deserialize)]
pub struct EmailRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct UpdateMeRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteAccountRequest {
    #[serde(default)]
    pub password: Option<String>,
    pub confirm_email: String,
}

#[derive(Serialize)]
pub struct SessionRow {
    pub id: Uuid,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
    #[serde(with = "rfc3339")]
    pub last_seen_at: Ts,
    #[serde(with = "rfc3339")]
    pub expires_at: Ts,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub current: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiles
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone, Copy, Default)]
pub struct ProfileStats {
    pub solved: i64,
    pub attempted: i64,
    pub favourites: i64,
}

#[derive(Serialize, Clone)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub avatar: String,
    pub color: String,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
    #[serde(with = "rfc3339")]
    pub last_seen_at: Ts,
    pub stats: ProfileStats,
}

#[derive(Deserialize)]
pub struct ProfileInput {
    pub name: String,
    pub avatar: String,
    pub color: String,
}

#[derive(Deserialize)]
pub struct ProfilePatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress, playlists
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ProgressEntry {
    pub status: String,
    pub favourite: bool,
    pub attempts: i32,
    #[serde(with = "rfc3339::option")]
    pub solved_at: Option<Ts>,
    #[serde(with = "rfc3339")]
    pub updated_at: Ts,
}

impl ProgressEntry {
    /// The entry of a problem nobody has touched: "to do" is the absence of a row.
    pub fn untouched() -> Self {
        Self {
            status: "todo".into(),
            favourite: false,
            attempts: 0,
            solved_at: None,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

#[derive(Serialize)]
pub struct ProgressSnapshot {
    pub entries: BTreeMap<String, ProgressEntry>,
    pub stats: ProfileStats,
}

#[derive(Deserialize)]
pub struct SetStatusRequest {
    pub status: String,
}

#[derive(Deserialize)]
pub struct SetFavouriteRequest {
    pub favourite: bool,
}

#[derive(Serialize)]
pub struct Playlist {
    pub id: Uuid,
    pub name: String,
    pub slugs: Vec<String>,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
}

#[derive(Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
    #[serde(default)]
    pub slugs: Vec<String>,
}

#[derive(Deserialize)]
pub struct RenamePlaylistRequest {
    pub name: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Content
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct Language {
    pub id: String,
    pub label: String,
    pub ext: String,
    pub syntax: String,
    pub comment: String,
    pub order: i32,
    pub runnable: bool,
}

#[derive(Serialize, Clone)]
pub struct CatalogProblem {
    pub slug: String,
    pub title: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub tier: Tier,
    pub leetcode_url: String,
    pub viz: bool,
    pub langs: Vec<String>,
    pub premium: bool,
}

#[derive(Serialize, Clone)]
pub struct CatalogCategory {
    pub name: String,
    pub problems: Vec<CatalogProblem>,
}

#[derive(Serialize, Clone)]
pub struct TierInfo {
    pub id: Tier,
    pub title: &'static str,
    pub count: usize,
    pub animated: usize,
}

#[derive(Serialize, Clone)]
pub struct Catalog {
    pub content_version: String,
    pub tiers: Vec<TierInfo>,
    pub categories: Vec<CatalogCategory>,
    pub languages: Vec<Language>,
    pub total: usize,
    pub animated: usize,
}

#[derive(Serialize, Clone)]
pub struct TestCaseView {
    pub name: String,
    pub input: InputMap,
    pub stdin: String,
    pub expected: String,
    pub edge: bool,
}

#[derive(Serialize, Clone)]
pub struct ProblemSource {
    pub lang: String,
    pub code: String,
    pub tag_lines: BTreeMap<String, usize>,
    pub line_tags: BTreeMap<String, String>,
    pub starter: String,
    pub harness: Option<String>,
    pub synthesized: bool,
}

#[derive(Serialize, Clone)]
pub struct RelatedProblem {
    pub slug: String,
    pub title: String,
    pub difficulty: Difficulty,
}

#[derive(Serialize, Clone)]
pub struct Problem {
    pub slug: String,
    pub title: String,
    pub category: String,
    pub difficulty: Difficulty,
    pub tier: Tier,
    pub description: String,
    pub approach: String,
    pub complexity: String,
    pub leetcode_url: String,
    pub inputs: Vec<InputField>,
    pub default_input: InputMap,
    pub default_fields: BTreeMap<String, String>,
    pub tests: Vec<TestCaseView>,
    pub hints: Vec<String>,
    pub related: Vec<RelatedProblem>,
    pub guide_topics: Vec<String>,
    pub has_trace: bool,
    pub premium: bool,
    pub locked: bool,
    pub sources: Vec<ProblemSource>,
}

#[derive(Deserialize, Default)]
pub struct TraceRequest {
    #[serde(default)]
    pub fields: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub input: Option<InputMap>,
}

#[derive(Serialize)]
pub struct TraceResponse<'a> {
    pub input: &'a InputMap,
    pub trace: &'a Trace,
}

// ─────────────────────────────────────────────────────────────────────────────
// Guide
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GuideOpRow {
    pub op: String,
    pub big: String,
    pub note: String,
}

#[derive(Serialize)]
pub struct GuideTopic {
    pub id: String,
    pub title: String,
    pub kind: &'static str,
    pub emoji: String,
    pub what: Vec<String>,
    pub complexity: Vec<GuideOpRow>,
    pub syntax: BTreeMap<String, String>,
    pub notes: Vec<String>,
}

#[derive(Serialize)]
pub struct CheatRow {
    pub topic: String,
    pub code: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct CheatSection {
    pub name: String,
    pub rows: Vec<CheatRow>,
}

#[derive(Serialize)]
pub struct Guide {
    pub topics: Vec<GuideTopic>,
    pub cheatsheet: Vec<CheatSection>,
    pub by_category: BTreeMap<String, Vec<String>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Runs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RunRequest {
    pub slug: String,
    pub lang: String,
    pub code: String,
    pub mode: String,
    pub kind: String,
    #[serde(default)]
    pub stdin: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: String,
    pub stdin: String,
    pub expected: String,
    pub actual: String,
    pub detail: String,
    pub duration_ms: u64,
    pub edge: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CompileInfo {
    pub ok: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// What a run produced, as stored in `submissions.result` and returned inline.
#[derive(Serialize, Deserialize, Clone)]
pub struct RunOutput {
    pub status: String,
    pub compile: Option<CompileInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<Option<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timed_out: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<Vec<TestResult>>,
    pub passed: i32,
    pub total: i32,
    pub duration_ms: u64,
}

#[derive(Serialize)]
pub struct RunResult {
    pub id: Uuid,
    pub slug: String,
    pub lang: String,
    pub kind: String,
    pub mode: String,
    #[serde(flatten)]
    pub output: RunOutput,
    pub progress: ProgressEntry,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
}

#[derive(Serialize)]
pub struct SubmissionSummary {
    pub id: Uuid,
    pub slug: String,
    pub lang: String,
    pub kind: String,
    pub mode: String,
    pub status: String,
    pub passed: i32,
    pub total: i32,
    pub duration_ms: i32,
    #[serde(with = "rfc3339")]
    pub created_at: Ts,
}

#[derive(Serialize)]
pub struct Submission {
    #[serde(flatten)]
    pub summary: SubmissionSummary,
    pub code: String,
    pub result: serde_json::Value,
}

#[derive(Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Drafts
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct Draft {
    pub lang: String,
    pub code: String,
    #[serde(with = "rfc3339")]
    pub updated_at: Ts,
}

#[derive(Deserialize)]
pub struct SaveDraftRequest {
    pub code: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct Totals {
    pub solved: i64,
    pub attempted: i64,
    pub favourites: i64,
    pub submissions: i64,
}

#[derive(Serialize)]
pub struct TierStat {
    pub tier: Tier,
    pub title: &'static str,
    pub solved: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct CategoryStat {
    pub category: String,
    pub solved: usize,
    pub attempted: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct DifficultyStat {
    pub difficulty: Difficulty,
    pub solved: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct Streak {
    pub current: u32,
    pub longest: u32,
    pub active_today: bool,
}

#[derive(Serialize)]
pub struct ActivityDay {
    pub day: String,
    pub runs: i32,
    pub solved: i32,
}

#[derive(Serialize)]
pub struct Stats {
    pub totals: Totals,
    pub by_tier: Vec<TierStat>,
    pub by_category: Vec<CategoryStat>,
    pub by_difficulty: Vec<DifficultyStat>,
    pub streak: Streak,
    pub activity: Vec<ActivityDay>,
    pub recent: Vec<SubmissionSummary>,
}

// ─────────────────────────────────────────────────────────────────────────────
// AI
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct AiChatRequest {
    pub slug: String,
    pub lang: String,
    pub mode: String,
    pub messages: Vec<AiMessage>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub run_context: Option<String>,
}

#[derive(Serialize)]
pub struct AiStatus {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub daily_limit: u32,
    pub used_today: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Billing, import, admin
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub interval: String,
}

#[derive(Serialize)]
pub struct RedirectUrl {
    pub url: String,
}

#[derive(Deserialize)]
pub struct ImportEntry {
    pub status: String,
    #[serde(default)]
    pub favourite: bool,
    #[serde(default)]
    pub attempts: i32,
}

#[derive(Deserialize)]
pub struct ImportPlaylist {
    pub name: String,
    #[serde(default)]
    pub slugs: Vec<String>,
}

#[derive(Deserialize)]
pub struct ImportRequest {
    #[serde(default)]
    pub entries: BTreeMap<String, ImportEntry>,
    #[serde(default)]
    pub playlists: Vec<ImportPlaylist>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub progress_rows: usize,
    pub playlists: usize,
}

#[derive(Serialize)]
pub struct AdminOverview {
    pub users: i64,
    pub users_pro: i64,
    pub signups_7d: i64,
    pub runs_24h: i64,
    pub ai_requests_24h: i64,
    pub content_errors: Vec<String>,
}
