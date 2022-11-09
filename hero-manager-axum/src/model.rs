use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize, Deserializer};
use validator::Validate;

#[derive(Serialize, Deserialize, Validate, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(sqlx::FromRow)]
pub struct Hero {
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    #[serde(deserialize_with = "deserialize_abilities", default)]
    #[validate(length(max = 5))]
    pub abilities: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(sqlx::FromRow)]
pub struct IdentifyableHero {
    pub id: i64,
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub inner_hero: Hero,
    pub version: i32,
}

fn deserialize_abilities<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let concat_abilities = Option::<String>::deserialize(deserializer)?;
    match concat_abilities {
        Some(abilities) => Ok(Some(abilities.split(',').map(|s| s.trim().to_string()).collect())),
        None => Ok(None),
    }
}
