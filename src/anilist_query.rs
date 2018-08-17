use reqwest::Client;
use serde_json::from_str;
use std::collections::HashMap;

use database;
use query_structs;

pub fn get_id(username: &str) -> Option<query_structs::User> {
    // Construct query to anilist GraphQL to find corresponding id for username
    let query = USER_QUERY.replace("{}", username.as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);
    let client = Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::UserResponse = from_str(&res_text).unwrap();

    // If the username was valid, there will be some data, else there will be errors
    match json.data.user {
        Some(user) => {
            database::update_user_profile(user.clone());
            Some(user)
        }
        None => {
            error!(
                "user_name={} was not found in anilist/external database",
                username
            );
            None
        }
    }
}

pub fn get_lists(id: i32) -> Vec<query_structs::MediaList> {
    let query = LIST_QUERY.replace("{}", id.to_string().as_ref());
    let mut body = HashMap::new();
    body.insert("query", query);

    let client = Client::new();
    let mut res = client.post(ANILSIT_URL).json(&body).send().unwrap();
    let res_text = res.text().unwrap();
    let json: query_structs::ListResponse = from_str(&res_text).unwrap();
    json.data.media_list_collection.lists.clone()
}

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
