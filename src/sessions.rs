use argon2::{Argon2, PasswordHash, PasswordVerifier};
use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::{Deserialize, Serialize};
use serde_email::Email;
use sqlx::SqlitePool;
use tower_sessions::Session;
use tracing::info;

#[derive(Deserialize, Debug)]
pub struct LoginParams {
    redirect: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginPage {
    redirect: Option<String>,
    error: Option<&'static str>,
}
pub async fn login_page(Query(params): Query<LoginParams>) -> LoginPage {
    LoginPage {
        redirect: params.redirect,
        error: None,
    }
}

#[derive(Deserialize, Debug)]
pub struct Input {
    email: Email,
    password: String,
    redirect: Option<String>,
}

struct DbUser {
    id: i64,
    name: String,
    email: String,
    age: i64,
    password_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i64,
}

pub async fn login(
    session: Session,
    State(db): State<SqlitePool>,
    Form(input): Form<Input>,
) -> axum::response::Result<Response> {
    info!("Sign in request from user: {}", input.email);

    let mut conn = db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let email = input.email.to_string();
    let user = sqlx::query_as!(DbUser, "SELECT * FROM users WHERE email = ?", email)
        .fetch_one(&mut *conn)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let valid = verify_password(input.password, user.password_hash);

    session
        .insert(
            "user",
            SessionUser {
                id: user.id,
                name: user.name,
                email: user.email,
                age: user.age,
            },
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let redirect = match &input.redirect {
        Some(uri) => Redirect::to(uri),
        None => Redirect::to("/"),
    };

    if valid {
        Ok(redirect.into_response())
    } else {
        Ok(LoginPage {
            redirect: input.redirect,
            error: Some("Invalid login"),
        }
        .into_response())
    }
}

fn verify_password(password: String, hash: String) -> bool {
    let argon2 = Argon2::default();
    argon2
        .verify_password(password.as_bytes(), &PasswordHash::new(&hash).unwrap())
        .is_ok()
}
