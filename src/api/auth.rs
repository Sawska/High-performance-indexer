//! Email + password accounts with cookie sessions.
//!
//! The cookie holds a random token; the `sessions` table holds only its
//! SHA-256, so a leaked database does not hand out live sessions. The cookie is
//! HttpOnly and SameSite=Lax, and every state-changing endpoint takes a JSON
//! body, which a cross-site form cannot send -- that is the CSRF defence.
//!
//! `COOKIE_SECURE=true` adds the Secure flag; leave it off for plain-HTTP
//! localhost.

use std::sync::LazyLock;

use argon2::Argon2;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::ApiError;

const COOKIE: &str = "session";
const SESSION_DAYS: i64 = 30;
const MIN_PASSWORD: usize = 8;
const MAX_PASSWORD: usize = 256;

#[derive(Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
}

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: String,
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn session_cookie(token: String) -> Cookie<'static> {
    let secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "true");
    Cookie::build((COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(time::Duration::days(SESSION_DAYS))
        .build()
}

/// Hashing is deliberately slow, so it runs off the async workers.
async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(ApiError::internal)?
    .map_err(ApiError::internal)
}

async fn verify_password(password: String, hash: String) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash).map_err(ApiError::internal)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(ApiError::internal)?
}

/// Verified against when the email is unknown, so a miss costs the same time
/// as a wrong password and does not reveal which emails have accounts.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(b"not a real password", &salt)
        .expect("argon2 with default params")
        .to_string()
});

fn normalize(creds: Credentials) -> Result<(String, String), ApiError> {
    let email = creds.email.trim().to_lowercase();
    if email.len() > 254 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "enter a valid email",
        ));
    }
    let len = creds.password.chars().count();
    if !(MIN_PASSWORD..=MAX_PASSWORD).contains(&len) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("password must be {MIN_PASSWORD}-{MAX_PASSWORD} characters"),
        ));
    }
    Ok((email, creds.password))
}

async fn start_session(pool: &PgPool, user_id: i64, jar: CookieJar) -> Result<CookieJar, ApiError> {
    let token = new_token();
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(Utc::now() + Duration::days(SESSION_DAYS))
        .execute(pool)
        .await?;
    Ok(jar.add(session_cookie(token)))
}

pub async fn register(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(creds): Json<Credentials>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    let (email, password) = normalize(creds)?;
    let hash = hash_password(password).await?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2)
         ON CONFLICT (email) DO NOTHING
         RETURNING id, email",
    )
    .bind(&email)
    .bind(&hash)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::new(StatusCode::CONFLICT, "an account with this email exists"))?;

    let jar = start_session(&pool, user.id, jar).await?;
    Ok((jar, Json(user)))
}

pub async fn login(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(creds): Json<Credentials>,
) -> Result<(CookieJar, Json<User>), ApiError> {
    let email = creds.email.trim().to_lowercase();

    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, email, password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&pool)
            .await?;

    let (user, hash) = match row {
        Some((id, email, hash)) => (Some(User { id, email }), hash),
        None => (None, DUMMY_HASH.clone()),
    };

    let ok = verify_password(creds.password, hash).await?;
    let user = match (user, ok) {
        (Some(u), true) => u,
        _ => {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "wrong email or password",
            ));
        }
    };

    let jar = start_session(&pool, user.id, jar).await?;
    Ok((jar, Json(user)))
}

pub async fn logout(State(pool): State<PgPool>, jar: CookieJar) -> Result<CookieJar, ApiError> {
    if let Some(c) = jar.get(COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_token(c.value()))
            .execute(&pool)
            .await?;
    }
    Ok(jar.remove(Cookie::build(COOKIE).path("/")))
}

async fn current_user(pool: &PgPool, jar: &CookieJar) -> Result<Option<User>, ApiError> {
    let Some(c) = jar.get(COOKIE) else {
        return Ok(None);
    };
    Ok(sqlx::query_as::<_, User>(
        "SELECT u.id, u.email
            FROM sessions s
            JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(hash_token(c.value()))
    .fetch_optional(pool)
    .await?)
}

pub async fn me(State(pool): State<PgPool>, jar: CookieJar) -> Result<Json<User>, ApiError> {
    current_user(&pool, &jar)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "not signed in"))
}

/// Rejects the request with 401 unless it carries a live session.
pub async fn require_user(
    State(pool): State<PgPool>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let user = current_user(&pool, &jar)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "not signed in"))?;
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}
