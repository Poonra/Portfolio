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
        
    
    let cli = Cli::parse(); //clap parse argv to our struct

    match cli.cmd {
        Command::Init =>{
            let pool = db::connect_from_env().await?;
            db::init_schema(&pool).await?;
            println!("Database ready");
        }
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
                println!("Transaction add user={user} {side} {qty} {symbol} @ {price} fee={fee} ts={ts}");
            }
            TxnCmd::List { user, limit } => {
                println!("txn list: user={user} limit={limit}");
            }
        },

        Command::Sync { offline } => {
            println!("sync: offline={offline}");
        }

        Command::Positions { user } => {
            println!("positions for user={user}");
        }

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

