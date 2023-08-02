// Data access layer for our sample
//
// This sample module implements a simple data access layer with sqlx and Postgres.
// A core idea of this sample implementation is the use of a trait with automock.
// With that, web API handler functions can be unit-tested.
//
// Note that this sample focusses on web APIs and Axum. Therefore, no integration
// tests have been developed with sqlx (read more about that topic at
// https://docs.rs/sqlx/latest/sqlx/attr.test.html).

use crate::model::{Hero, IdentifyableHero};
use axum::async_trait;
#[cfg(test)]
use mockall::automock;
use sqlx::PgPool;
use tracing::error;

/// Represents primary key and version data for a hero
pub struct HeroPkVersion {
    pub id: i64,
    pub version: i32,
}

/// Logs an sqlx error
pub fn log_error(e: sqlx::Error) -> sqlx::Error {
    error!("Failed to execute SQL statement: {:?}", e);
    e
}

/// Repository for maintaining heroes in the DB
#[cfg_attr(test, automock)]
#[async_trait]
pub trait HeroesRepositoryTrait {
    /// Deletes all heroes from the DB
    async fn cleanup(&self) -> Result<(), sqlx::error::Error>;

    /// Gets a list of heroes from the DB filted by name
    async fn get_by_name(&self, name: &str) -> Result<Vec<IdentifyableHero>, sqlx::error::Error>;

    /// Insert a new hero in the DB
    async fn insert(&self, hero: &Hero) -> Result<HeroPkVersion, sqlx::error::Error>;
}

/// Implementation of the heroes repository
pub struct HeroesRepository(pub PgPool);

#[async_trait]
impl HeroesRepositoryTrait for HeroesRepository {
    async fn cleanup(&self) -> Result<(), sqlx::error::Error> {
        sqlx::query("DELETE FROM heroes").execute(&self.0).await?;
        Ok(())
    }

    async fn get_by_name(&self, name: &str) -> Result<Vec<IdentifyableHero>, sqlx::error::Error> {
        sqlx::query_as::<_, IdentifyableHero>("SELECT * FROM heroes WHERE name LIKE $1")
            .bind(name)
            .fetch_all(&self.0)
            .await
    }

    async fn insert(&self, hero: &Hero) -> Result<HeroPkVersion, sqlx::error::Error> {
        let pk: (i64, i32) = sqlx::query_as(
            r#"
            INSERT INTO heroes (first_seen, name, can_fly, realname, abilities)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, version"#,
        )
        .bind(hero.first_seen)
        .bind(&hero.name)
        .bind(hero.can_fly)
        .bind(&hero.realname)
        .bind(&hero.abilities)
        .fetch_one(&self.0)
        .await?;
        Ok(HeroPkVersion {
            id: pk.0,
            version: pk.1,
        })
    }
}
