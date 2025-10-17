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

    sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS transactions (
        id        INTEGER PRIMARY KEY AUTOINCREMENT,
        user      TEXT    NOT NULL,
        asset_id  INTEGER NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
        side      TEXT    NOT NULL CHECK (side IN ('buy','sell','dividend','deposit','withdrawal')),
        qty       REAL    NOT NULL,
        price     REAL    NOT NULL,
        fee       REAL    NOT NULL DEFAULT 0,
        ts        TEXT    NOT NULL,   -- ISO 8601 (UTC)
        note      TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_txn_user_ts ON transactions(user, ts DESC);
    "#
).execute(pool).await?;


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

pub async fn find_asset_id_by_symbol(pool: &Db, symbol: &str) -> anyhow::Result<i64>{
    let rows = sqlx::query(
        r#"SELECT id FROM assets WHERE symbol = ?1"#
    )
    .bind(symbol)
    .fetch_all(pool)
    .await?;

        match rows.len() {
        1 => Ok(rows[0].get::<i64, _>("id")),
        0 => anyhow::bail!("asset '{symbol}' not found (add it first with `asset add`)"),
        _ => anyhow::bail!("symbol '{symbol}' is ambiguous (multiple asset types)"),
    }


}

pub async fn insert_txn(
    pool: &Db,
    user:&str,
    symbol:&str,
    side: &str,
    qty:f64,
    price:f64,
    fee:f64,
    ts:&str,
    note: Option<&str>,
) -> anyhow::Result<i64>{

    let s =side.to_lowercase();
    let ok =["buy","sell","dividend","deposit","withdrawal"].contains(&s.as_str());
    if !ok { anyhow::bail!("side must be one of: buy|sell|dividend|deposit|withdrawal"); }
    if qty <= 0.0 { anyhow::bail!("qty must be > 0"); }
    if price < 0.0 || fee < 0.0 { anyhow::bail!("price/fee must be >= 0"); }

    let asset_id = find_asset_id_by_symbol(pool, symbol).await?;

    let res = sqlx::query(
        r#"
            INSERT INTO transactions (user, asset_id,side,qty,price,fee,ts,note)
            VALUES(?,?,?,?,?,?,?,?)
            "#
    )
    .bind(user)
    .bind(asset_id)
    .bind(s)
    .bind(qty)
    .bind(price)
    .bind(fee)
    .bind(ts)
    .bind(note)
    .execute(pool)
    .await?;

    Ok(res.last_insert_rowid())
}

#[derive(Debug)]
pub struct TxnRow{
    pub id: i64,
    pub ts: String,
    pub user: String,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub note: Option<String>,
}

pub async fn list_txns(pool: &Db, user: &str,limit:usize) -> anyhow::Result<Vec<TxnRow>>{
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.ts, t.user, a.symbol, t.side, t.qty, t.price, t.fee, t.note
        FROM transactions t
        JOIN assets a ON a.id = t.asset_id
        WHERE t.user = ?1
        ORDER BY t.ts DESC
        LIMIT ?2
        "#
    )
    .bind(user)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;


    Ok(rows.into_iter().map(|r| TxnRow {
        id:    r.get("id"),
        ts:    r.get("ts"),
        user:  r.get("user"),
        symbol:r.get("symbol"),
        side:  r.get("side"),
        qty:   r.get("qty"),
        price: r.get("price"),
        fee:   r.get("fee"),
        note:  r.try_get("note").ok(),
    }).collect())

}


