#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
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

use rocket::http::Status;
use rocket::response::status::Accepted;
use rocket::response::status::NotFound;
use rocket_contrib::Json;
use std::thread;

mod anilist_models;
mod anilist_query;
mod database;
mod models;
mod schema;

#[get("/user/<username>")]
fn user(
    username: String,
    conn: database::DbConn,
) -> Result<Json<models::ResponseList>, NotFound<String>> {
    match database::get_list(&username, conn) {
        Some(list) => Ok(Json(list)),
        None => Err(NotFound("User or list not found".to_owned())),
    }
}

#[put("/user/<username>")]
fn update(
    username: String,
    rocket_con: database::DbConn,
) -> Result<Accepted<String>, NotFound<String>> {
    match anilist_query::get_id(&username, rocket_con) {
        Some(an_user_tup) => {
            let connection = database::update_user_profile(an_user_tup.0, an_user_tup.1);
            let db_result = database::get_user(&username, connection.unwrap());
            let user_option = db_result.0;
            let db_res_con = db_result.1;
            match user_option {
                Some(u) => {
                    thread::spawn(move || database::update_entries(u.user_id, db_res_con));
                    Ok(Accepted(Some("Added to the queue".to_owned())))
                }
                None => match anilist_query::get_id(&username, db_res_con) {
                    Some(an_user_tup_2) => {
                        thread::spawn(move || {
                            database::update_entries(an_user_tup_2.0.id, an_user_tup_2.1)
                        });
                        Ok(Accepted(Some("Added to the queue".to_owned())))
                    }
                    None => Err(NotFound("User not found".to_owned())),
                },
            }
        }
        None => return Err(NotFound("User not found".to_owned())),
    }
}

fn main() {
    if setup_logger().is_err() {
        std::process::abort()
    }

    rocket::ignite()
        .manage(database::init_pool())
        .mount("/", routes![update, user])
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
        }).level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("trx.log")?)
        .apply()?;
    Ok(())
}
