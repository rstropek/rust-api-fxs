use axum::async_trait;
use sqlx::{pool::PoolConnection, Postgres, PgPool};
use tracing::error;
#[cfg(test)]
use mockall::{automock, mock, predicate::*};

use crate::model::{Hero, IdentifyableHero};

pub struct HeroPkVersion {
    pub id: i64,
    pub version: i32,
}

pub struct DatabaseConnection(pub PoolConnection<Postgres>);

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HeroesRepositoryTrait {
    async fn cleanup(&self) -> Result<(), sqlx::error::Error>;
    async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<IdentifyableHero>, sqlx::error::Error>;
    async fn insert(
        &self,
        hero: &Hero,
    ) -> Result<HeroPkVersion, sqlx::error::Error>;
}

pub struct HeroesRepository(pub PgPool);

impl HeroesRepository {
    async fn get_connection(&self) -> Result<DatabaseConnection, sqlx::error::Error> {
        Ok(DatabaseConnection(self.0.acquire().await.map_err(|e| {
            error!("Failed to acquire connection from pool: {}", e);
            e
        })?))
    }
}

#[async_trait]
impl HeroesRepositoryTrait for HeroesRepository {
    async fn cleanup(&self) -> Result<(), sqlx::error::Error> {
        let mut conn = self.get_connection().await?;
        sqlx::query("DELETE FROM heroes").execute(&mut conn.0).await?;
        Ok(())
    }

    async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<IdentifyableHero>, sqlx::error::Error> {
        let mut conn = self.get_connection().await?;
        //let mut conn = self.0;
        sqlx::query_as::<_, IdentifyableHero>("SELECT * FROM heroes WHERE name LIKE $1")
            .bind(name)
            .fetch_all(&mut conn.0)
            .await
    }

    async fn insert(
        &self,
        hero: &Hero,
    ) -> Result<HeroPkVersion, sqlx::error::Error> {
        //let mut conn = self.0;
        let mut conn = self.get_connection().await?;
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
        .fetch_one(&mut conn.0)
        .await?;
        Ok(HeroPkVersion {
            id: pk.0,
            version: pk.1,
        })
    }
}
