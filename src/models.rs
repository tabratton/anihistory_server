/*
 * Copyright (c) 2018, Tyler Bratton
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use chrono::NaiveDate;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
//#[table_name = "users"]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub avatar_s3: String,
    pub avatar_anilist: String,
}

#[derive(Debug, Clone)]
//#[table_name = "anime"]
pub struct Anime {
    pub anime_id: i32,
    pub description: String,
    pub cover_s3: String,
    pub cover_anilist: String,
    pub average: Option<i16>,
    pub native: Option<String>,
    pub romaji: Option<String>,
    pub english: Option<String>,
}

#[derive(Debug, Clone)]
//#[table_name = "lists"]
pub struct ListItem {
    pub user_id: i32,
    pub anime_id: i32,
    pub user_title: Option<String>,
    pub start_day: Option<NaiveDate>,
    pub end_day: Option<NaiveDate>,
    pub score: Option<i16>,
}

#[derive(Debug, Clone)]
//#[table_name = "lists"]
pub struct ListItemMap {
    pub user: User,
    pub anime: Anime,
    pub list_item: ListItem,
}

#[derive(Serialize, Deserialize)]
pub struct RestResponse {
    pub users: ResponseList,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseList {
    pub id: String,
    pub avatar: String,
    pub list: Vec<ResponseItem>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseItem {
    pub user_title: Option<String>,
    pub start_day: Option<NaiveDate>,
    pub end_day: Option<NaiveDate>,
    pub score: Option<i16>,
    pub average: Option<i16>,
    pub native: Option<String>,
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub description: String,
    pub cover: String,
    pub id: i32,
}
