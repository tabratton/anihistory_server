/*
 * Copyright (c) 2018, Tyler Bratton
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::{anilist_models, anilist_query, models};
use chrono::NaiveDate;
use dotenv::dotenv;
use log::{error, info};
use reqwest::blocking::get;
use rocket_contrib::databases::postgres::{Connection, TlsMode};
use rusoto_core::Region;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use std::io::Read;
use std::{env, thread, panic};

// Only used for upload_to_s3 because of spawned threads and I didn't want to make the connection
// pool work with that.
fn establish_connection() -> Connection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let result = Connection::connect(database_url.as_ref(), TlsMode::None);
    match result {
        Ok(connection) => connection,
        Err(error) => {
            error!("error connecting to {}. Error: {}", database_url, error);
            panic!();
        }
    }
}

pub fn get_list(name: &str, connection: &postgres::Connection) -> Option<models::RestResponse> {
    let stmt = connection
	  .prepare_cached("SELECT u.user_id, u.name, u.avatar_s3, u.avatar_anilist, a.anime_id, a\
	  .description, a.cover_s3, a.cover_anilist, a.average, a.native, a.romaji, a.english, l\
	  .user_title, l.start_day, l.end_day, l.score FROM lists as l INNER JOIN users as u ON l\
	  .user_id=u.user_id INNER JOIN anime as a ON l.anime_id=a.anime_id WHERE u.name = $1")
	  .unwrap();

    let results = stmt.query(&[&name]);

    match results {
        Ok(result) => {
            let mut database_list: Vec<models::ListItemMap> = Vec::with_capacity(result.len());
            for row in result.iter() {
                let user = models::User {
                    user_id: row.get(0),
                    name: row.get(1),
                    avatar_s3: row.get(2),
                    avatar_anilist: row.get(3),
                };

                let anime = models::Anime {
                    anime_id: row.get(4),
                    description: row.get(5),
                    cover_s3: row.get(6),
                    cover_anilist: row.get(7),
                    average: row.get(8),
                    native: row.get(9),
                    romaji: row.get(10),
                    english: row.get(11),
                };

                let list_item = models::ListItem {
                    user_id: row.get(0),
                    anime_id: row.get(4),
                    user_title: row.get(12),
                    start_day: row.get(13),
                    end_day: row.get(14),
                    score: row.get(15),
                };

                database_list.push(models::ListItemMap {
                    user,
                    anime,
                    list_item,
                });
            }

            if database_list.len() > 0 {
                let mut response_items: Vec<models::ResponseItem> =
                    Vec::with_capacity(database_list.len());
                for list_item in database_list.clone() {
                    let item = models::ResponseItem {
                        user_title: list_item.list_item.user_title,
                        start_day: list_item.list_item.start_day,
                        end_day: list_item.list_item.end_day,
                        score: list_item.list_item.score,
                        average: list_item.anime.average,
                        native: list_item.anime.native,
                        romaji: list_item.anime.romaji,
                        english: list_item.anime.english,
                        description: list_item.anime.description,
                        cover: list_item.anime.cover_s3,
                        id: list_item.anime.anime_id,
                    };

                    response_items.push(item);
                }
                Some(models::RestResponse {
                    users: models::ResponseList {
                        id: database_list[0].user.name.clone(),
                        avatar: database_list[0].user.avatar_s3.clone(),
                        list: response_items,
                    },
                })
            } else {
                None
            }
        }
        Err(error) => {
            error!(
                "error getting list for user_name={}. Error: {}",
                name, error
            );
            None
        }
    }
}

pub fn update_user_profile(user: anilist_models::User, connection: &Connection) {
    let ext = get_ext(&user.avatar.large);

    let new_user = models::User {
        user_id: user.id.clone(),
        name: user.name.clone(),
        avatar_s3: format!(
            "https://s3.amazonaws.com/anihistory-images/assets/images/user_{}.{}",
            user.id, ext
        ),
        avatar_anilist: user.avatar.large.clone(),
    };

    let stmt = connection.prepare_cached("INSERT INTO users (user_id, name, avatar_s3, avatar_anilist) VALUES ($1, $2, $3, $4) ON CONFLICT (user_id) DO UPDATE SET name = excluded.name, avatar_s3 = excluded.avatar_s3, avatar_anilist = excluded.avatar_anilist").unwrap();

    let result = stmt.execute(&[
        &new_user.user_id,
        &new_user.name,
        &new_user.avatar_s3,
        &new_user.avatar_anilist,
    ]);

    // Download their avatar and upload to S3.
    let mut content = Vec::new();
    download_image(&mut content, &user.avatar.large);
    upload_to_s3(ImageTypes::User, user.id, ext.clone(), content);

    match result {
        Ok(_) => (),
        Err(error) => {
            error!("error saving user={:?}. Error: {}", new_user, error);
            ()
        }
    }
}

pub fn delete_entries(lists: Vec<anilist_models::MediaList>, id: i32) {
    let connection = establish_connection();
    let mut used_lists = Vec::new();

    for mut list in lists {
        if list.name.to_lowercase().contains("completed")
            || list.name.to_lowercase().contains("watching")
        {
            list.entries
                .sort_unstable_by(|a, b| a.media.id.cmp(&b.media.id));
            used_lists.push(list.clone());
        }
    }

    let stmt = connection.prepare_cached("SELECT user_id, anime_id, user_title, start_day, end_day, score FROM lists WHERE user_id = $1").unwrap();

    let user_db_list_result = stmt.query(&[&id]);

    match user_db_list_result {
        Ok(rows) => {
            for row in rows.iter() {
                let list_item = models::ListItem {
                    user_id: row.get(0),
                    anime_id: row.get(1),
                    user_title: row.get(2),
                    start_day: row.get(3),
                    end_day: row.get(4),
                    score: row.get(5),
                };

                let mut found = false;

                for list in used_lists.clone() {
                    let result = list
                        .entries
                        .binary_search_by(|e| e.media.id.cmp(&list_item.anime_id));
                    match result {
                        Ok(_) => found = true,
                        Err(_) => {}
                    }
                }

                if !found {
                    println!("deleting anime:{}", list_item.anime_id);
                    let stmt = connection
                        .prepare_cached("DELETE FROM lists WHERE user_id = $1 AND anime_id = $2")
                        .unwrap();

                    let delete_result = stmt.execute(&[&list_item.user_id, &list_item.anime_id]);

                    if delete_result.is_err() {
                        error!(
                            "error deleting list_entry={:?}. Error: {}",
                            row,
                            delete_result.expect_err("?")
                        );
                    }
                }
            }
        }
        Err(err) => {
            error!("error retrieving list for user_id={:?}. Error: {}", id, err);
        }
    }
}

pub fn update_entries(id: i32) {
    let lists: Vec<anilist_models::MediaList> = anilist_query::get_lists(id);

    delete_entries(lists.clone(), id);
    let connection = establish_connection();

    for list in lists {
        if list.name.to_lowercase().contains("completed")
            || list.name.to_lowercase().contains("watching")
        {
            for entry in list.entries {
                let ext = get_ext(&entry.media.cover_image.large);

                let new_anime = models::Anime {
                    anime_id: entry.media.id,
                    description: entry.media.description,
                    cover_s3: format!(
                        "https://s3.amazonaws.com/anihistory-images/assets/images/anime_{}.{}",
                        entry.media.id, ext
                    ),
                    cover_anilist: entry.media.cover_image.large.clone(),
                    average: entry.media.average_score,
                    native: entry.media.title.native,
                    romaji: entry.media.title.romaji,
                    english: entry.media.title.english,
                };

                let stmt = connection.prepare_cached("INSERT INTO anime (anime_id, description, cover_s3, cover_anilist, average, native, romaji, english) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (anime_id) DO UPDATE SET description = excluded.description, cover_s3 = excluded.cover_s3, cover_anilist = excluded.cover_anilist, average = excluded.average, native = excluded.native, romaji = excluded.romaji, english = excluded.english").unwrap();

                let anime_result = stmt.execute(&[
                    &new_anime.anime_id,
                    &new_anime.description,
                    &new_anime.cover_s3,
                    &new_anime.cover_anilist,
                    &new_anime.average,
                    &new_anime.native,
                    &new_anime.romaji,
                    &new_anime.english,
                ]);

                match anime_result {
                    Ok(_) => {
                        // Download cover images and upload to S3.
                        let mut content = Vec::new();
                        download_image(&mut content, &entry.media.cover_image.large);
                        let closure_id = entry.media.id.clone();
                        let closure_ext = ext.clone();
                        thread::spawn(move || {
                            upload_to_s3(ImageTypes::Anime, closure_id, closure_ext, content)
                        });
                    }
                    Err(error) => {
                        error!("error saving anime={:?}. Error: {}", new_anime, error);
                    }
                }

                let start = construct_date(entry.started_at);
                let end = construct_date(entry.completed_at);

                let new_list = models::ListItem {
                    user_id: id,
                    anime_id: entry.media.id,
                    user_title: entry.media.title.user_preferred,
                    start_day: start,
                    end_day: end,
                    score: entry.score_raw,
                };

                let stmt = connection.prepare_cached("INSERT INTO lists (user_id, anime_id, user_title, start_day, end_day, score) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (user_id, anime_id) DO UPDATE SET user_title = excluded.user_title, start_day = excluded.start_day, end_day = excluded.end_day, score = excluded.score").unwrap();

                let list_result = stmt.execute(&[
                    &new_list.user_id,
                    &new_list.anime_id,
                    &new_list.user_title,
                    &new_list.start_day,
                    &new_list.end_day,
                    &new_list.score,
                ]);

                if list_result.is_err() {
                    error!(
                        "error saving list_entry={:?}. Error: {}",
                        new_list,
                        list_result.expect_err("?")
                    );
                }
            }
        }
    }
    info!("Database updated for user_id={}", id);
}

fn upload_to_s3(prefix: ImageTypes, id: i32, ext: String, content: Vec<u8>) {
    let image_prefix: String;
    match prefix {
        ImageTypes::Anime => image_prefix = "anime".to_owned(),
        ImageTypes::User => image_prefix = "user".to_owned(),
    };

    let client = S3Client::new(Region::UsEast1);
    let bucket_name = "anihistory-images";
    let mime = naive_mime(&ext);
    let key = format!("assets/images/{}_{}.{}", image_prefix, id, ext);

    let put_request = PutObjectRequest {
        bucket: bucket_name.to_owned(),
        key: key.clone(),
        body: Some(content.into()),
        content_type: Some(mime),
        acl: Some("public-read".to_owned()),
        ..PutObjectRequest::default()
    };

    match client.put_object(put_request).sync() {
        Ok(_) => (),
        Err(error) => {
            error!(
                "error uploading assets/images/{}_{}.{} to S3. Error: {}",
                image_prefix, id, ext, error
            );
        }
    }
}

fn construct_date(date: anilist_models::Date) -> Option<NaiveDate> {
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

fn download_image(content: &mut Vec<u8>, url: &String) {
    let mut resp = get(url).unwrap();
    resp.read_to_end(content).unwrap();
}

fn get_ext(url: &String) -> String {
    let link_parts: Vec<&str> = url.split('/').collect();
    let splitted: Vec<&str> = link_parts[link_parts.len() - 1].split(".").collect();
    splitted[1].to_owned()
}

fn naive_mime(ext: &String) -> String {
    if ext.contains("jp") {
        "image/jpeg".to_owned()
    } else {
        format!("image/{}", ext)
    }
}

enum ImageTypes {
    Anime,
    User,
}
