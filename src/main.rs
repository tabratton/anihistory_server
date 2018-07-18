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

use rocket_contrib::Json;
use serde_json::Value;
use std::thread;

mod anilist_query;
mod database;
mod models;
mod query_structs;
mod schema;

// User passes in username, first query to find userID, then query with
// found ID.
#[get("/user/<username>")]
fn user(username: String) -> Json<Value> {
    let user = anilist_query::get_id(username);
    match user {
        Some(u) => Json(json!(u)),
        None => Json(json!({"success": false})),
    }

    // TODO: Query database for this user's list.
    // SELECT u.user_id, u.name, u.avatar_s3, a.anime_id, l.user_title, a.english, a.romaji, a.native,
    // a.description, a.cover_s3, a.average, l.start_day, l.end_day, l.score
    // FROM anime AS a INNER JOIN (lists AS l INNER JOIN users AS u ON l.user_id=u.user_id)
    // ON a.anime_id=l.anime_id WHERE u.user_id=<userid>;
}

// Update all entries for a user
#[put("/user/<username>")]
fn update(username: String) -> &'static str {
    let user = anilist_query::get_id(username);
    match user {
        Some(u) => {
            thread::spawn(move || database::update_entries(u.id));
            "Added to the queue"
        }
        None => "User not found",
    }
}

fn main() {
    rocket::ignite().mount("/", routes![update, user]).launch();
}
