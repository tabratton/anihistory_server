use chrono::NaiveDate;
use schema::anime;
use schema::lists;
use schema::users;

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[table_name = "users"]
pub struct User {
    pub user_id: i32,
    pub name: String,
    pub avatar_s3: String,
    pub avatar_anilist: String,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset)]
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

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset)]
#[table_name = "lists"]
pub struct List {
    pub user_id: i32,
    pub anime_id: i32,
    pub user_title: Option<String>,
    pub start_day: Option<NaiveDate>,
    pub end_day: Option<NaiveDate>,
    pub score: Option<i16>,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseList {
    pub user: User,
    pub items: Vec<ResponseItem>,
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
    pub cover_s3: String,
}
