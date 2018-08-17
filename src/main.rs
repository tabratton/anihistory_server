#[macro_use]
extern crate serde_derive;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate diesel;
extern crate actix_web;
extern crate chrono;
extern crate dotenv;
extern crate fern;
#[macro_use]
extern crate log;

use actix_web::{http, middleware, server, App, Either, HttpResponse, Responder, Path};
use std::thread;

mod anilist_query;
mod database;
mod models;
mod query_structs;
mod schema;

fn user(req: Path<(String,)>) -> impl Responder {
  match database::get_list(&req.0) {
        Some(list) => Either::A(HttpResponse::Ok().json(list)),
        None => Either::B(HttpResponse::BadRequest().body("No list data")),
    }
}

fn update(req: Path<(String,)>) -> impl Responder {
    match database::get_user(&req.0) {
        Ok(user) => {
            thread::spawn(move || database::update_entries(user.user_id));
            Either::A(HttpResponse::Ok().body("Added to the queue"))
        }
        Err(_) => match anilist_query::get_id(&req.0) {
            Some(user) => {
                thread::spawn(move || database::update_entries(user.id));
                Either::A(HttpResponse::Ok().body("Added to the queue"))
            }
            None => Either::B(HttpResponse::BadRequest().body("User not found")),
        },
    }
}

fn main() {
    if setup_logger().is_err() {
        std::process::abort()
    }

    server::new(|| {
        App::new()
            .middleware(middleware::Logger::default())
            .route("/user/{username}", http::Method::GET, user)
            .route("/user/{username}", http::Method::PUT, update)
    }).bind("127.0.0.1:5000")
    .unwrap()
    .run();
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        }).level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("trx.log")?)
        .apply()?;
    Ok(())
}
