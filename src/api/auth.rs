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

#[cfg(test)]
mod unit {
    use super::*;

    fn creds(email: &str, password: &str) -> Credentials {
        Credentials {
            email: email.into(),
            password: password.into(),
        }
    }

    #[test]
    fn normalize_trims_and_lowercases_email() {
        let (email, password) = normalize(creds("  Bob@Example.COM ", "hunter22!")).ok().unwrap();
        assert_eq!(email, "bob@example.com");
        assert_eq!(password, "hunter22!");
    }

    #[test]
    fn normalize_rejects_bad_emails() {
        for bad in ["", "bob", "@example.com", "bob@", &format!("{}@x.io", "a".repeat(260))] {
            let err = normalize(creds(bad, "long enough")).err().expect(bad);
            assert_eq!(err.status, StatusCode::BAD_REQUEST, "{bad}");
        }
    }

    #[test]
    fn normalize_bounds_password_length_in_characters() {
        assert!(normalize(creds("a@b.c", "seven77")).is_err());
        assert!(normalize(creds("a@b.c", "eight888")).is_ok());
        assert!(normalize(creds("a@b.c", "пароль12")).is_ok());
        assert!(normalize(creds("a@b.c", &"x".repeat(MAX_PASSWORD))).is_ok());
        assert!(normalize(creds("a@b.c", &"x".repeat(MAX_PASSWORD + 1))).is_err());
    }

    #[test]
    fn tokens_are_random_and_stored_only_as_sha256() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(hash_token(&a), hash_token(&a));
        assert_ne!(hash_token(&a), a);
        assert_eq!(
            hash_token("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn session_cookie_is_http_only_lax_and_site_wide() {
        let c = session_cookie("tok".into());
        assert_eq!(c.name(), COOKIE);
        assert_eq!(c.http_only(), Some(true));
        assert_eq!(c.same_site(), Some(SameSite::Lax));
        assert_eq!(c.path(), Some("/"));
        assert_eq!(c.max_age(), Some(time::Duration::days(SESSION_DAYS)));
    }

    #[tokio::test]
    async fn password_hash_round_trips() {
        let hash = hash_password("correct horse".into()).await.ok().unwrap();
        assert!(verify_password("correct horse".into(), hash.clone()).await.ok().unwrap());
        assert!(!verify_password("wrong horse".into(), hash).await.ok().unwrap());
        assert!(!verify_password("x".into(), DUMMY_HASH.clone()).await.ok().unwrap());
    }
}
