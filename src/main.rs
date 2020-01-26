/*
 * Copyright (c) 2018, Tyler Bratton
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![feature(proc_macro_hygiene, decl_macro)]

use rocket::get;
use rocket::http::Method;
use rocket::post;
use rocket::response::status::Accepted;
use rocket::response::status::NotFound;
use rocket::routes;
use rocket_contrib::database;
use rocket_contrib::databases::postgres;
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use rocket_cors::Error;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::thread;

mod anilist_models;
mod anilist_query;
mod database;
mod models;

#[database("postgres_connection")]
pub struct PgDbConn(postgres::Connection);

#[get("/users/<username>")]
fn user(
    username: String,
    database_conn: PgDbConn,
) -> Result<Json<models::RestResponse>, NotFound<String>> {
    match database::get_list(username.as_ref(), &database_conn) {
        Some(list) => Ok(Json(list)),
        None => Err(NotFound("User or list not found".to_owned())),
    }
}

#[post("/users/<username>")]
fn update(username: String, database_conn: PgDbConn) -> Result<Accepted<String>, NotFound<String>> {
    match anilist_query::get_id(username.as_ref()) {
        Some(user) => {
            database::update_user_profile(user.clone(), &database_conn);
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
