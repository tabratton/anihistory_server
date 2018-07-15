extern crate chrono;

use self::chrono::NaiveDate;
use super::schema::anime;
use super::schema::lists;
use super::schema::users;

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name = "users"]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub avatar_s3: String,
    pub avatar_anilist: String,
}

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name = "anime"]
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

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name = "lists"]
pub struct List {
    pub user_id: i32,
    pub anime_id: i32,
    pub user_title: Option<String>,
    pub start_day: Option<NaiveDate>,
    pub end_day: Option<NaiveDate>,
    pub score: Option<i16>,
}
