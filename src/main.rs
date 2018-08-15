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
    // temp response (want to fail on database query because user info should be loaded before
    // getting list
    match database::get_user(info.0.clone()) {
        //	Ok(user) => Either::A(HttpResponse::Ok().json(database::get_list(user))),
        Ok(user) => Either::A(HttpResponse::Ok().json(user)),
        Err(_) => Either::B(HttpResponse::BadRequest().body("User not found")),
    }

    // TODO: Query database for this user's list.
    //SELECT lists.user_title, anime.english, anime.romaji, anime.native, anime.description, anime.cover_s3, anime.average, lists.start_day, lists.end_day, lists.score
    //FROM anime
    //INNER JOIN lists
    //  ON lists.anime_id=anime.anime_id
    //INNER JOIN users
    //  ON lists.user_id=users.user_id
    //WHERE users.user_id=<user_id>>;
}

// Update all entries for a user
fn update(info: Path<(String,)>) -> impl Responder {
    match database::get_user(info.0.clone()) {
        Ok(user) => {
            thread::spawn(move || database::update_entries(user.user_id));
            Either::A(HttpResponse::Ok().body("Added to the queue"))
        }
        Err(_) => match anilist_query::get_id(info.0.clone()) {
            Some(user) => {
                thread::spawn(move || database::update_entries(user.id));
                Either::A(HttpResponse::Ok().body("Added to the queue"))
            }
            None => Either::B(HttpResponse::BadRequest().body("User not found")),
        },
    }
}

fn main() {
    println!("Starting server...");
    server::new(|| {
        App::new()
            .resource("/user/{username}", |r| {
                r.method(http::Method::GET).with(user)
            }).resource("/updateUser/{username}", |r| {
                r.method(http::Method::PUT).with(update)
            })
    }).bind("127.0.0.1:5000")
    .unwrap()
    .run();
}
