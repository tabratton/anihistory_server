/*
 * Copyright (c) 2018, Tyler Bratton
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate dotenv;
extern crate fern;
extern crate reqwest;
extern crate rocket_cors;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate rusoto_signature;
extern crate serde;
extern crate serde_json;

use rocket::http::Method;
use rocket::response::status::Accepted;
use rocket::response::status::NotFound;
use rocket_contrib::databases::diesel as rocket_diesel;
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use rocket_cors::Error;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::thread;

mod anilist_models;
mod anilist_query;
mod database;
mod models;
mod schema;

#[database("postgres_connection")]
pub struct PgDbConn(rocket_diesel::PgConnection);

#[get("/users/<username>")]
fn user(username: String, conn: PgDbConn) -> Result<Json<models::RestResponse>, NotFound<String>> {
    match database::get_list(username.as_ref(), &conn) {
        Some(list) => Ok(Json(list)),
        None => Err(NotFound("User or list not found".to_owned())),
    }
}

#[post("/users/<username>")]
fn update(username: String, rocket_con: PgDbConn) -> Result<Accepted<String>, NotFound<String>> {
    match anilist_query::get_id(username.as_ref()) {
        Some(user) => {
            database::update_user_profile(user.clone(), &rocket_con);
            thread::spawn(move || database::update_entries(user.id));
            Ok(Accepted(Some("Added to the queue".to_owned())))
        }
        None => Err(NotFound("User not found".to_owned())),
    }
}

fn main() -> Result<(), Error> {
    if setup_logger().is_err() {
        std::process::abort()
    }

    let allowed_origins = AllowedOrigins::some_exact(&[
        "http://localhost:4200",
        "https://anihistory.moe",
        "https://www.anihistory.moe",
    ]);
  
    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;

    rocket::ignite()
        .mount("/", StaticFiles::from("static"))
        .mount("/", routes![update, user])
        .attach(cors)
        .attach(PgDbConn::fairing())
        .launch();

    Ok(())
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
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("trx.log")?)
        .apply()?;
    Ok(())
}
