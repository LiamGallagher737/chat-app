use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use serde_email::Email;
use sqlx::SqlitePool;
use tracing::info;

#[derive(Template, Default)]
#[template(path = "signup.html")]
pub struct SignupPage {
    error: Option<&'static str>,
}
pub async fn signup_page() -> SignupPage {
    SignupPage::default()
}

#[derive(Deserialize, Debug)]
pub struct Input {
    name: String,
    age: u8,
    email: Email,
    password: String,
}

#[derive(Template)]
#[template(path = "success.html")]
pub struct SuccessPage {
    name: String,
    email: String,
}

pub async fn post_user(
    State(db): State<SqlitePool>,
    Form(input): Form<Input>,
) -> axum::response::Result<Response> {
    info!("New sign up from user: {input:?}");
    let mut conn = db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let email = input.email.to_string();

    let already_exists = sqlx::query!("SELECT * FROM users WHERE email = ?", email)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();
    if already_exists {
        return Ok(SignupPage {
            error: Some("User already exists"),
        }
        .into_response());
    }

    let hash = hash_password(input.password).ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query!(
        "INSERT INTO users (name, email, age, password_hash) VALUES ( ?, ?, ?, ? )",
        input.name,
        email,
        input.age,
        hash,
    )
    .execute(&mut *conn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(SuccessPage {
        name: input.name,
        email,
    }
    .into_response())
}

fn hash_password(password: String) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2: Argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .ok()?
        .to_string();
    Some(hash)
}
