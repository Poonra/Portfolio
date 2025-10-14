use sqlx::{Sqlite, Pool};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;

pub type Db = Pool<Sqlite>;

// Connect using DATABASE_URL from .env
pub async fn connect_from_env()->anyhow::Result<Db> {
    let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL not set (put in.env)");
    let pool= SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

    Ok(pool)
}
// create schema
pub async fn init_schema(pool: &Db) -> anyhow::Result<()> {
    // asset: id, Symbol, Type, currency, name
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS assets(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        symbol TEXT NOT NULL,
        asset_type TEXT NOT NULL CHECK(asset_type IN ('stock', 'etf', 'crypto')),
        currency TEXT NOT NULL DEFAULT 'USD',
        name TEXT,
        UNIQUE(symbol,asset_type)
    );
    "#).execute(pool).await?;


    Ok(())
    
}
// Insert a new asset, return row id
pub async fn insert_asset(
    pool: &Db,
    symbol: &str,
    asset_type: &str,
    currency: &str,
    name: Option<&str>,

) -> anyhow::Result<i64>{
    let res = sqlx::query(
        r#"
        INSERT INTO assets(symbol, asset_type,currency, name)
        VALUES (?1, ?2, ?3, ?4)
        "#)
    .bind(symbol)
    .bind(asset_type)
    .bind(currency)
    .bind(name)
    .execute(pool)
    
    .await?;

    Ok(res.last_insert_rowid())

}
// row type for listings asset
#[derive(Debug)]
pub struct AssetRow {
    pub id:i64,
    pub symbol: String,
    pub asset_type: String,
    pub currency: String,
    pub name: Option<String>
}

// fetch all asset
pub async fn list_assets(pool:&Db) -> anyhow::Result<Vec<AssetRow>> {
    let rows = sqlx::query(
        r#"
        SELECT id, symbol, asset_type, currency, name
        FROM assets
        ORDER BY symbol
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
    .map(|r| AssetRow {
        id: r.get::<i64, _>("id"),
        symbol: r.get::<String, _>("symbol"),
        asset_type: r.get::<String, _>("asset_type"),
        currency: r.get::<String, _>("currency"),
        name: r.try_get::<String, _>("name").ok(),
    })
    .collect())
}