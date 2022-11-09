use sqlx::{
    pool::PoolConnection,
    Postgres,
};

use crate::model::{IdentifyableHero, Hero};

pub struct HeroPkVersion {
    pub id: i64,
    pub version: i32,
}

pub struct DatabaseConnection(pub PoolConnection<Postgres>);

pub async fn cleanup(DatabaseConnection(conn): DatabaseConnection) -> Result<(), sqlx::error::Error> {
    let mut conn = conn;
    sqlx::query("DELETE FROM heroes").execute(&mut conn).await?;
    Ok(())
}

pub async fn get_by_name(
    DatabaseConnection(conn): DatabaseConnection,
    name: &str,
) -> Result<Vec<IdentifyableHero>, sqlx::error::Error> {
    let mut conn = conn;
    sqlx::query_as::<_, IdentifyableHero>("SELECT * FROM heroes WHERE name LIKE $1")
        .bind(name)
        .fetch_all(&mut conn)
        .await
}

pub async fn insert(
    DatabaseConnection(conn): DatabaseConnection,
    hero: &Hero,
) -> Result<HeroPkVersion, sqlx::error::Error> {
    let mut conn = conn;
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
    .fetch_one(&mut conn)
    .await?;
    Ok(HeroPkVersion {
        id: pk.0,
        version: pk.1,
    })
}
