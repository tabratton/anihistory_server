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

use actix_web::{http, server, App, Either, HttpResponse, Path, Responder};
use std::thread;

mod anilist_query;
mod database;
mod models;
mod query_structs;
mod schema;

// User passes in username, first query to find userID, then query with
// found ID.
fn user(info: Path<(String,)>) -> impl Responder {
    let user = anilist_query::get_id(info.0.clone());
    match user {
        Some(u) => Either::A(HttpResponse::Ok().json(u)),
        None => Either::B(HttpResponse::BadRequest().body("User not found")),
    }

    // TODO: Query database for this user's list.
    // SELECT u.user_id, u.name, u.avatar_s3, a.anime_id, l.user_title, a.english, a.romaji, a.native,
    // a.description, a.cover_s3, a.average, l.start_day, l.end_day, l.score
    // FROM anime AS a INNER JOIN (lists AS l INNER JOIN users AS u ON l.user_id=u.user_id)
    // ON a.anime_id=l.anime_id WHERE u.user_id=<userid>;
}

// Update all entries for a user
fn update(info: Path<(String,)>) -> impl Responder {
    let user = anilist_query::get_id(info.0.clone());
    match user {
        Some(u) => {
            thread::spawn(move || database::update_entries(u.id));
            Either::A(HttpResponse::Ok().body("Added to the queue"))
        }
        None => Either::B(HttpResponse::BadRequest().body("User not found")),
    }
}

fn main() {
    println!("Starting server...");
    server::new(|| {
        App::new()
            .resource("/user/{username}", |r| {
                r.method(http::Method::GET).with(user)
            })
		    .resource("/updateUser/{username}", |r| {
                r.method(http::Method::PUT).with(update)
            })
    })
	.bind("127.0.0.1:5000")
    .unwrap()
    .run();
}
