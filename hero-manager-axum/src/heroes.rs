use axum::{response::IntoResponse, Json, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

use crate::{data::{self, DatabaseConnection}, problem_details::ProblemDetail};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddHeroDto {
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    #[serde(deserialize_with = "deserialize_abilities", default)]
    pub abilities: Option<Vec<String>>,
}

fn deserialize_abilities<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where D: Deserializer<'de> {
    let concat_abilities = Option::<String>::deserialize(deserializer)?;
    match concat_abilities {
        Some(abilities) => Ok(Some(abilities.split(',').map(|s| s.trim().to_string()).collect())),
        None => Ok(None)
    }
}

pub async fn get_all_heroes(DatabaseConnection(conn): DatabaseConnection) -> impl IntoResponse {
    todo!("get_all_heroes")
}

pub async fn insert_hero(conn: DatabaseConnection, Json(hero): Json<AddHeroDto>) -> impl IntoResponse {
    if let Some(abilities) = &hero.abilities {
        if abilities.len() > 5 {
            return ProblemDetail::UnprocessableEntity("Too many abilities").into_response();
        }
    }

    let hero = data::NewHero {
        first_seen: hero.first_seen,
        name: hero.name,
        can_fly: hero.can_fly,
        realname: hero.realname,
        abilities: hero.abilities,
    };

    data::insert(conn, &hero).await.unwrap();

    StatusCode::OK.into_response()
}
