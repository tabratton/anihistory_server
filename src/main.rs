#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate diesel;
extern crate chrono;
extern crate dotenv;
extern crate fern;
#[macro_use]
extern crate log;

use fern::colors::{Color, ColoredLevelConfig};
use rocket::http::Status;
use rocket::response::status::Accepted;
use rocket::response::Failure;
use rocket_contrib::Json;
use std::thread;

mod anilist_models;
mod anilist_query;
mod database;
mod models;
mod schema;

#[get("/user/<username>")]
fn user(username: String) -> Result<Json<models::ResponseList>, Status> {
    match database::get_list(&username) {
        Some(list) => Ok(Json(list)),
        None => Err(Status::NotFound),
    }
}

#[put("/user/<username>")]
fn update(username: String) -> Result<Accepted<String>, Status> {
    match database::get_user(&username) {
        Ok(user) => {
            thread::spawn(move || database::update_entries(user.user_id));
            Ok(Accepted(Some("Added to the queue".to_owned())))
        }
        Err(_) => match anilist_query::get_id(&username) {
            Some(user) => {
                thread::spawn(move || database::update_entries(user.id));
                Ok(Accepted(Some("Added to the queue".to_owned())))
            }
            None => Err(Status::NotFound),
        },
    }
}

fn main() {
    if setup_logger().is_err() {
        std::process::abort()
    }

    rocket::ignite().mount("/", routes![update, user]).launch();
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.target(),
                message
            ))
        }).level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("trx.log")?)
        .apply()?;
    Ok(())
}
