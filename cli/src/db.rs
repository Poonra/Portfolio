use sqlx::{Sqlite, Pool};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;
use std::collections::HashMap;



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

    sqlx::query(
        r#"
    CREATE TABLE IF NOT EXISTS prices (
        asset_id INTERGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
        date    TEXT    NOT NULL,
        close   REAL    NOT NULL,
        source  TEXT    NOT NULL,
        PRIMARY KEY (asset_id,date)
    );

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

pub async fn upsert_price(
    pool: &Db,
    asset_id:i64,
    date: &str,
    close:f64,
    source:&str,
) -> anyhow::Result<()>{
    sqlx::query(
        r#"
                INSERT INTO price (asset_id,date, close, source)
                VALUES(?,?,?,?)
                ON CONFLICT(asset_id,date)
                DO UPDATE SET close= excluded.close,source=excluded.source
                "#
    )
    .bind(asset_id)
    .bind(date)
    .bind(close)
    .bind(source)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn lastest_price(pool: &Db, asset_id: i64,) -> anyhow::Result<Option<(String,f64)>>{
    let row = sqlx::query(
        r#"
             SELECT date,close
             from prices
             WHERE asset_id = ?
             ORDER BY date DESC
             LIMIT 1
             "#
    )
        .bind(asset_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r|{
        let date: String = r.get("date");
        let close: f64 = r.get("close");
        (date,close)

    }))
}


#[derive(Debug)]
pub struct Position {
    #[allow(dead_code)]
    pub asset_id: i64,
    pub symbol: String,
    pub asset_type: String,
    pub currency: String,
    pub net_qty: f64,
    pub avg_cost: f64,
    pub last_price: Option<f64>,
    pub market_value: f64,
    pub unrealized_pl: f64,
}

// get avg cost
// buy add qtt +fee sell reduce nur qtt

pub async fn compute_position(pool: &Db, user:&str) -> anyhow::Result<Vec<Position>>{

        let txns = sqlx::query(
            r#"
                SELECT a.id as asset_id, a.symbol, a.asset_type, a.currency,
                t.side, t.qty, t.price, t.fee
                FROM transactions t
                JOIN assets a ON a.id = t.asset_id
                WHERE t.user = ?
                ORDER BY t.ts ASC
                "#
        )

            .bind(user)
            .fetch_all(pool)
            .await?;

        struct  Acc{
            symbol: String, asset_type:String, currency: String,
            buy_qty:f64, buy_cost:f64, sell_qty:f64,
        }

        let mut map: HashMap<i64, Acc> =  HashMap::new();

        for r in txns{
            let id: i64 = r.get("asset_id");
            let entry = map.entry(id).or_insert_with(|| Acc {
                symbol:   r.get("symbol"),
                asset_type: r.get("asset_type"),
                currency: r.get("currency"),
                buy_qty: 0.0, buy_cost: 0.0, sell_qty: 0.0,
            });
            let side: String = r.get("side");
            let qty:  f64 = r.get("qty");
            let price:f64 = r.get("price");
            let fee:  f64 = r.get("fee");

            match side.as_str(){
                "buy" => {entry.buy_qty += qty; entry.buy_cost += qty*price + fee;}
                "sell" => {entry.sell_qty += qty;}
                _=> {} // dividend/deposit/withdraw ignore cost + qty
            }
        }

    let mut out = Vec::new();
    for (asset_id,acc) in map{
        let net_qty =acc.buy_qty-acc.sell_qty;
        if net_qty.abs() < f64::EPSILON{
            continue;

        }
        let avg_cost = if acc.buy_qty >0.0 { acc.buy_cost/ acc.buy_qty} else{0.0};

        //lp = lastprice
        let lp = lastest_price(pool,asset_id).await?;
        let last_price = lp .as_ref().map(|(_,p)| *p);
        let market_value =last_price.unwrap_or(0.0) * net_qty.max(0.0);
        let unrealized_pl = match last_price{
            Some(p) => net_qty * (p - avg_cost),
            None => 0.0,
        };

        out.push(Position {
            asset_id,
            symbol: acc.symbol,
            asset_type: acc.asset_type,
            currency: acc.currency,
            net_qty,
            avg_cost,
            last_price,
            market_value,
            unrealized_pl,
        });
    }
    out.sort_by(|a,b| a.symbol.cmp(&b.symbol));
    Ok(out)
}
