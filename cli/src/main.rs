use clap::{Parser, Subcommand};
mod db; // load db.rs



///
#[derive(Parser)]
#[command(name="portfolio", version, about="Track stocks & crypto")]

struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]

enum Command{

    Init,
/// manage asset add/list
    Asset{
        #[command(subcommand)]
        action: AssetCmd,

    },

/// manage Transaction
    Txn{
        #[command(subcommand)]
        action: TxnCmd,
    },
/// sync prices(online/offline)
    Sync {
        //offline fixture
        #[arg(long)]
        offline: bool,


    },

/// Positions summary
    Positions{
        /// which userid or name
        #[arg(long,default_value = "you")]
        user: String,

    },
/// json summary
    Summary {
    #[arg(long,default_value = "you")]
    user: String,

    #[arg(long)]
    json: bool,

    },

}

#[derive(Subcommand)]

enum AssetCmd{
    /// add stock or asset
    Add{
        #[arg(long)] symbol: String,
        #[arg(long)] r#type: String, //stock etf crypto
        #[arg(long,default_value = "USD")] currency: String,
        #[arg(long)] name: Option<String>,


    },
    List,
}

#[derive(Subcommand)]
enum TxnCmd {
    /// add transaction
    Add{
        #[arg(long)] user: String,
        #[arg(long)] symbol: String,
        #[arg(long)] side: String,   // buy | sell | dividend | deposit | withdrawal
        #[arg(long)] qty: f64,
        #[arg(long)] price: f64,
        #[arg(long, default_value_t=0.0)] fee: f64,
        #[arg(long)] ts: String,
    },

    //list transaction
    List {
        #[arg(long, default_value="you")] user: String,
        #[arg(long, default_value_t=20)] limit: usize,
    },
}



#[tokio::main]

    async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok(); // read .env
       println!("DB CONNECT");

eprintln!("CWD = {}", std::env::current_dir()?.display());
eprintln!("DATABASE_URL = {:?}", std::env::var("DATABASE_URL"));

        
    
    let cli = Cli::parse(); //clap parse argv to our struct

    match cli.cmd {
        Command::Init =>{
            let pool = db::connect_from_env().await?;
            db::init_schema(&pool).await?;
            println!("Database ready");
        },
        Command::Asset { action } => match action{

            AssetCmd::Add { symbol, r#type, currency, name } => {
                let t = r#type.to_lowercase();
                let valid = ["stock","etf","crypto"].contains(&t.as_str());
                if !valid{
                    anyhow::bail!("[ERROR] type must be one of stock | etf | crypto")
                }
                let pool = db::connect_from_env().await?;
                let id = db::insert_asset(&pool, &symbol, &t, &currency, name.as_deref()).await?;
                println!("added asset id ={id}: {symbol} [{t}] {currency} {:?}", name);
            }
            AssetCmd::List => {
                let pool = db::connect_from_env().await?;
                let rows = db::list_assets(&pool).await?;

                if rows.is_empty(){
                    println!("no asset yet");
                } else {
                    println!("ID    SYMBOL  TYPE    CURRENCY    NAME");
                    for a in rows {
                        println!(
                            "{:<4} {:<7} {:<8} {:<9} {}",
                            a.id,
                            a.symbol,
                            a.asset_type,
                            a.currency,
                            a.name.unwrap_or_default()

                        );
                    }
                }
            }
        },

        Command::Txn{action}=> match action{


            TxnCmd::Add { user, symbol, side, qty, price, fee, ts } =>{
                let pool = db::connect_from_env().await?;
                db::init_schema(&pool).await?;
                let s = side.to_lowercase();
                let valid_sides = ["buy","sell","dividend","deposit","withdrawal"];
                if !valid_sides.contains(&s.as_str()) {
                    anyhow::bail!("[ERROR] side must be one of buy | sell | dividend | deposit | withdrawal");
                }


                let id = db::insert_txn(&pool, &user, &symbol, &side, qty, price, fee, &ts, None).await?;
                println!("added txn id={id}: {user} {side} {qty} {symbol} @ {price} fee={fee} ts={ts}");
            }
            TxnCmd::List { user, limit } => {
                let pool = db::connect_from_env().await?;
        let rows = db::list_txns(&pool, &user, limit).await?;
        if rows.is_empty() {
            println!("(no transactions yet)");
        } else {
            println!("ID   TS                        USER  SYMBOL  SIDE       QTY       PRICE      FEE   NOTE");
            for t in rows {
                println!("{:<4} {:<24} {:<5} {:<6} {:<10} {:>9.4} {:>10.4} {:>6.2} {}",
                    t.id, t.ts, t.user, t.symbol, t.side, t.qty, t.price, t.fee, t.note.unwrap_or_default()
                );
            }
        }
    }

        },

        Command::Sync { offline } => {
            let pool = db::connect_from_env().await?;
            db::upsert_price(&pool, "AAPL".parse()?, "2025-01-01T00:00:00Z", 199.99,"idk").await?;
            println!("sync: offline={offline}");
        },

        Command::Positions { user } => {
            let pool = db::connect_from_env().await?;
            db::init_schema(&pool).await?;
            let rows = db::compute_position(&pool, &user).await?;

            if rows.is_empty() {
                println!("no positions yet");
            } else {
                println!("SYMBOL    TYPE    QTY     AVG_COST    LAST_PRICE  MKT_VALUE UNRL_P/L  CUR");
                let mut tv = 0.0f64;
                let mut tpl = 0.0f64;
                for p in rows {
                    println!("{:<6} {:<7} {:>9.4} {:>10.4} {:>11.4} {:>10.2} {:>9.2} {:<3}",
                             p.symbol, p.asset_type, p.net_qty, p.avg_cost,
                             p.last_price.unwrap_or(0.0), p.market_value, p.unrealized_pl, p.currency
                    );
                    tv += p.market_value;
                    tpl += p.unrealized_pl;

                }
                println!("-----------------------------------------------------------------------------------------");
                println!("Total value={:.2}     P/L ={:.2}",tv,tpl);
            }
        },

        Command::Summary { user, json } => {
            if json {
                println!(r#"{{"total_value":0,"total_pl":0,"allocations":[],"user":"{user}"}}"#);
            } else {
                println!("summary for user={user} -> value=0, P/L=0");
            }
        }
    
    }

    Ok(())
}

