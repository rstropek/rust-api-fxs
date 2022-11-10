// Model for our sample
//
// We are maintaining a list of heroes. In this sample, we use identical model classes
// for web API and database access layer. This is for brevity. In more complicated projects,
// you will probably have different models for DB and web API.
//
// Nevertheless, the model has been chosen to demonstrate some interesting aspects:
// * Storing vectors in Postgres (abilities)
// * Customizing serde for properties (abilities)
// * Various serde macros (e.g. camelCase, flatten)
// * Auto-mapping DB columns to fiels (FromRow)

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::FromRow;
use validator::Validate;

#[derive(Clone, ValueEnum, Debug, Serialize, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

/// Application configuration
#[derive(Clone)]
pub struct AppConfiguration {
    pub version: &'static str,
    pub env: Environment
}

/// Represents a hero
#[derive(Serialize, Deserialize, Validate, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(FromRow, Default)]
pub struct Hero {
    pub first_seen: DateTime<Utc>,
    pub name: String,
    pub can_fly: bool,
    pub realname: Option<String>,
    #[serde(
        deserialize_with = "deserialize_abilities",
        serialize_with = "serialize_abilities",
        default
    )]
    #[validate(length(max = 5))]
    pub abilities: Option<Vec<String>>,
}

/// Represents a hero with primary key and version
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(FromRow, Default)]
pub struct IdentifyableHero {
    pub id: i64,
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub inner_hero: Hero,
    pub version: i32,
}

/// Deserialize vector of abilities from comma-separated string
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

/// Serialize vector of abilities into comma-separated string
fn serialize_abilities<S>(x: &Option<Vec<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(abilities) => serde::Serialize::serialize(&abilities.join(", "), serializer),
        None => serde::Serialize::serialize(&Option::<Vec<String>>::None, serializer),
    }
}

#[cfg(test)]
mod tests {
    // The following tests verify that abilities are serialized
    // and deserialized properly.

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct JustAbilities {
        #[serde(
            serialize_with = "super::serialize_abilities",
            deserialize_with = "super::deserialize_abilities",
            default
        )]
        pub abilities: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    struct DummyWithString {
        pub abilities: Option<String>,
    }

    #[test]
    fn serialize_abilities() {
        let serialized = serde_json::to_string_pretty(&JustAbilities {
            abilities: Some(vec!["a".to_string(), "b".to_string()]),
        })
        .unwrap();
        let result: DummyWithString = serde_json::from_str(&serialized).unwrap();
        assert_eq!("a, b", result.abilities.unwrap());
    }

    #[test]
    fn serialize_none() {
        let serialized = serde_json::to_string_pretty(&JustAbilities { abilities: None }).unwrap();
        let result: DummyWithString = serde_json::from_str(&serialized).unwrap();
        assert!(result.abilities.is_none());
    }

    #[test]
    fn deserialize_abilities() {
        let serialized: JustAbilities = serde_json::from_str("{ \"abilities\": \"a, b\" }").unwrap();
        assert_eq!(vec!["a", "b"], serialized.abilities.unwrap());
    }

    #[test]
    fn deserialize_none() {
        let serialized: JustAbilities = serde_json::from_str("{}").unwrap();
        assert!(serialized.abilities.is_none());
    }
}
