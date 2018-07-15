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
extern crate dotenv;

use self::models::User;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use rocket_contrib::Json;
use rusoto_core::Region;
use rusoto_s3::{PutObjectRequest, S3, S3Client};
use schema::users;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::io::Read;

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
    let collection: query_structs::MediaListCollection;
    match user {
        Some(u) => collection = get_list(u.user_id),
        None => return None,
    }

    let mut completed = collection.lists[0].clone();
    for list in collection.lists {
        if list.name == "Completed" {
            completed = list;
        }
    }

    Some(Json(json!(completed)))
}

fn get_id(username: String) -> Option<User> {
    // Construct query to anilist GraphQL to find corresponding id for username
    let query = USER_QUERY.replace("{}", username.as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);
    let client = reqwest::Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::UserResponse = serde_json::from_str(&res_text).unwrap();

    // If the username was valid, there will be some data, else there will be errors
    match json.data {
        Some(data) => {
            let mut content = Vec::new();
            let ext = download_image(&mut content, data.user.avatar.large);
            upload_to_s3("user".to_owned(), data.user.id, &ext, content);

            let connection = establish_connection();

            let new_user = User {
                user_id: data.user.id,
                name: data.user.name,
                avatar: format!(
                    "https://s3.amazonaws.com/anihistory-images/user_{}.{}",
                    data.user.id, ext
                ),
            };

            let user: User = diesel::insert_into(users::table)
                .values(&new_user)
                .on_conflict(users::user_id)
                .do_update()
                .set(&new_user)
                .get_result(&connection)
                .expect("Error saving new user");

            Some(user)
        }
        None => {
            let errors = json.errors.unwrap();
            let message = &errors[0].message;
            let status = &errors[0].status;
            println!("Error: Message - {}", message);
            println!("Error: Status - {}", status);
            None
        }
    }
}

fn get_list(id: i32) -> query_structs::MediaListCollection {
    let query = LIST_QUERY.replace("{}", id.to_string().as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);

    let client = reqwest::Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::ListResponse = serde_json::from_str(&res_text).unwrap();
    json.data.media_list_collection.clone()
}

fn main() {
    rocket::ignite().mount("/", routes![user]).launch();
}

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

fn upload_to_s3(prefix: String, id: i32, ext: &String, content: Vec<u8>) {
    let client = S3Client::simple(Region::UsEast1);
    let bucket_name = "anihistory-images";
  	let mime = naive_mime(&ext);
    let put_request = PutObjectRequest {
        bucket: bucket_name.to_owned(),
        key: format!("{}_{}.{}", prefix, id, ext),
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

fn naive_mime(ext: &String) -> String {
    if ext.contains("jp") {
        "image/jpeg".to_owned()
    } else {
        format!("image/{}", ext)
    }
}

fn download_image(content: &mut Vec<u8>, url: String) -> String {
    let mut resp = reqwest::get(&url).unwrap();
    resp.read_to_end(content).unwrap();

    let link_parts: Vec<&str> = url.split('/').collect();
    let splitted: Vec<&str> = link_parts[link_parts.len() - 1].split(".").collect();
    splitted[1].to_owned()
}
