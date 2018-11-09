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
extern crate serde;
extern crate serde_json;

use rocket::http::Method;
use rocket::response::status::Accepted;
use rocket::response::status::NotFound;
use rocket_contrib::json::Json;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::thread;

mod anilist_models;
mod anilist_query;
mod database;
mod models;
mod schema;

#[get("/users/<username>")]
fn user(
    username: String,
    conn: database::DbConn,
) -> Result<Json<models::RestResponse>, NotFound<String>> {
    match database::get_list(username.as_ref(), conn) {
        Some(list) => Ok(Json(list)),
        None => Err(NotFound("User or list not found".to_owned())),
    }
}

#[post("/users/<username>")]
fn update(
    username: String,
    rocket_con: database::DbConn,
) -> Result<Accepted<String>, NotFound<String>> {
    match anilist_query::get_id(username.as_ref()) {
        Some(user) => {
            database::update_user_profile(user.clone(), rocket_con);
            thread::spawn(move || database::update_entries(user.id));
            Ok(Accepted(Some("Added to the queue".to_owned())))
        }
        None => Err(NotFound("User not found".to_owned())),
    }
}

fn main() {
    if setup_logger().is_err() {
        std::process::abort()
    }

    let (allowed_origins, _failed_origins) = AllowedOrigins::some(&[
        "http://localhost:4200",
        "http://localhost",
        "https://anihistory.moe",
    ]);

    // You can also deserialize this
    let options = rocket_cors::Cors {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    };

    rocket::ignite()
        .manage(database::init_pool())
        .mount("/", routes![update, user])
        .attach(options)
        .launch();
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
