use actix_web::{Error, HttpRequest, HttpResponse, Responder};
use anyhow::Result;
use futures::future::{ready, Ready};
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub pk_id: i64,
    pub uk_username: String,
    pub uk_email: String,
    pub avatar: Option<String>,
    pub blog_url: Option<String>,
    pub introduce: Option<String>,
    pub github_uid: Option<String>,
    pub create_time: NaiveDateTime,
    pub update_time: NaiveDateTime,
    pub last_login_time: NaiveDateTime,
}

// implementation of Actix Responder for User struct so we can return User from action handler
impl Responder for User {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    #[inline]
    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();
        // create response and set content type
        ready(Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body)))
    }
}
