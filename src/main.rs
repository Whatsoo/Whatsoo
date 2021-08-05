extern crate serde_derive;
extern crate serde_json;

use actix_web::{middleware, App, HttpServer};
use anyhow::Result;
use dotenv::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::env;
use std::time::Duration;

mod common;
mod model;
mod route;
mod service;
mod sql;

#[actix_rt::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = MySqlPoolOptions::new()
        .max_connections(20)
        .min_connections(10)
        .max_lifetime(Some(Duration::from_millis(1800000)))
        .idle_timeout(Some(Duration::from_millis(600000)))
        .connect(&db_url)
        .await?;

    HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .wrap(middleware::Logger::default())
            .configure(route::init_all)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;
    Ok(())
}
