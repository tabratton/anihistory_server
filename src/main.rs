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

use self::models::Anime;
use self::models::List;
use self::models::User;
use chrono::NaiveDate;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use rocket_contrib::Json;
use rusoto_core::Region;
use rusoto_s3::{HeadObjectRequest, PutObjectRequest, S3, S3Client};
use schema::anime;
use schema::lists;
use schema::users;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::io::Read;
use std::thread;

pub mod models;
mod query_structs;
pub mod schema;

static ANILSIT_URL: &'static str = "https://graphql.anilist.co";

static LIST_QUERY: &'static str = "query {
    MediaListCollection(userId: {}, type: ANIME) {
      lists {
        name
        entries {
          ...mediaListEntry
        }
      }
    }
  }

  fragment mediaListEntry on MediaList {
    scoreRaw: score(format: POINT_100)
    startedAt {
      year
      month
      day
    }
    completedAt {
      year
      month
      day
    }
    media {
	  id
      title {
        userPreferred
        english
        romaji
        native
      }
      description(asHtml: true)
      coverImage {
        large
      }
      averageScore
      siteUrl
      }
    }";

static USER_QUERY: &'static str = "query {
  	User(name: \"{}\") {
	  id
      name
      avatar {
        large
      }
	}
  }";

// User passes in username, first query to find userID, then query with
// found ID.
#[get("/user/<username>")]
fn user(username: String) -> Option<Json<Value>> {
    let user = get_id(username);
    //    let collection: query_structs::MediaListCollection;
    //    match user {
    //        Some(u) => update_entries(u.user_id),
    //        None => return None,
    //    }
    //
    //    let mut completed = collection.lists[0].clone();
    //    for list in collection.lists {
    //        if list.name == "Completed" {
    //            completed = list;
    //        }
    //    }
    match user {
        Some(u) => Some(Json(json!(u))),
        None => Some(Json(json!({"success": false}))),
    }
}

// Update all entries for a user
#[put("/user/<username>")]
fn update(username: String) -> Json<Value> {
    let user = get_id(username);
    match user {
        Some(u) => {
            update_entries(u.id);
            Json(json!({"success": true}))
        }
        None => Json(json!({"success": false})),
    }
}

fn get_id(username: String) -> Option<query_structs::User> {
    // Construct query to anilist GraphQL to find corresponding id for username
    let query = USER_QUERY.replace("{}", username.as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);
    let client = reqwest::Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::UserResponse = serde_json::from_str(&res_text).unwrap();

    // If the username was valid, there will be some data, else there will be errors
    match json.data.user {
        Some(user) => {
            // Download their avatar and upload to S3.
            let mut content = Vec::new();
            let ext = download_image(&mut content, &user.avatar.large);
            upload_to_s3("user".to_owned(), user.id, ext.clone(), content, true);

            // Connect to DB and upsert user entry.
            let connection = establish_connection();

            let new_user = User {
                user_id: user.id.clone(),
                name: user.name.clone(),
                avatar: format!(
                    "https://s3.amazonaws.com/anihistory-images/user_{}.{}",
                    user.id, ext
                ),
            };

            diesel::insert_into(users::table)
                .values(&new_user)
                .on_conflict(users::user_id)
                .do_update()
                .set(&new_user)
                .execute(&connection)
                .expect("Error saving new user");

            Some(user)
        }
        None => None,
    }
}

fn update_entries(id: i32) {
    let query = LIST_QUERY.replace("{}", id.to_string().as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);

    let client = reqwest::Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::ListResponse = serde_json::from_str(&res_text).unwrap();

    let lists = json.data.media_list_collection.lists.clone();

    for list in lists {
        if list.name == "Completed" || list.name == "Watching" {
            for entry in list.entries {
                // Download cover images and upload to S3.
                let mut content = Vec::new();
                let ext = download_image(&mut content, &entry.media.cover_image.large);
                let closure_id = entry.media.id.clone();
                let closure_ext = ext.clone();
                thread::spawn(move || {
                    upload_to_s3("anime".to_owned(), closure_id, closure_ext, content, false)
                });

                // Connect to DB and upsert anime and list entries.
                let connection = establish_connection();

                let new_anime = Anime {
                    anime_id: entry.media.id,
                    description: entry.media.description,
                    cover: format!(
                        "https://s3.amazonaws.com/anihistory-images/anime_{}.{}",
                        entry.media.id, ext
                    ),
                    average: entry.media.average_score,
                    native: entry.media.title.native,
                    romaji: entry.media.title.romaji,
                    english: entry.media.title.english,
                };

                diesel::insert_into(anime::table)
                    .values(&new_anime)
                    .on_conflict(anime::anime_id)
                    .do_update()
                    .set(&new_anime)
                    .execute(&connection)
                    .expect("Error saving new anime");

                let start = construct_date(entry.started_at);
                let end = construct_date(entry.completed_at);

                let new_list = List {
                    user_id: id,
                    anime_id: entry.media.id,
                    user_title: entry.media.title.user_preferred,
                    start_day: start,
                    end_day: end,
                    score: entry.score_raw,
                };

                diesel::insert_into(lists::table)
                    .values(&new_list)
                    .on_conflict((lists::anime_id, lists::user_id))
                    .do_update()
                    .set(&new_list)
                    .execute(&connection)
                    .expect("Error saving new anime");
            }
        }
    }
}

fn construct_date(date: query_structs::Date) -> Option<NaiveDate> {
    match date.year {
        Some(year) => match date.month {
            Some(month) => match date.day {
                Some(day) => Some(NaiveDate::from_ymd(year, month as u32, day as u32)),
                None => None,
            },
            None => None,
        },
        None => None,
    }
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

fn upload_to_s3(prefix: String, id: i32, ext: String, content: Vec<u8>, replace: bool) {
    let client = S3Client::simple(Region::UsEast1);
    let bucket_name = "anihistory-images";
    let mime = naive_mime(&ext);
    let key = format!("{}_{}.{}", prefix, id, ext);

    let head_request = HeadObjectRequest {
        bucket: bucket_name.to_owned(),
        key: key.clone(),
        ..HeadObjectRequest::default()
    };

    let exists: bool;

    match client.head_object(&head_request).sync() {
        Ok(_) => exists = true,
        Err(_) => exists = false,
    }

    if exists && replace || !exists {
        let put_request = PutObjectRequest {
            bucket: bucket_name.to_owned(),
            key: key.clone(),
            body: Some(content),
            content_type: Some(mime),
            acl: Some("public-read".to_owned()),
            ..PutObjectRequest::default()
        };

        match client.put_object(&put_request).sync() {
            Ok(_) => {
                println!("{}_{}.{}", prefix, id, ext);
            }
            Err(error) => {
                println!("Error: {}", error);
            }
        }
    }
}

fn naive_mime(ext: &String) -> String {
    if ext.contains("jp") {
        "image/jpeg".to_owned()
    } else {
        format!("image/{}", ext)
    }
}

fn download_image(content: &mut Vec<u8>, url: &String) -> String {
    let mut resp = reqwest::get(url).unwrap();
    resp.read_to_end(content).unwrap();

    let link_parts: Vec<&str> = url.split('/').collect();
    let splitted: Vec<&str> = link_parts[link_parts.len() - 1].split(".").collect();
    splitted[1].to_owned()
}

fn main() {
    rocket::ignite().mount("/", routes![update, user]).launch();
}
