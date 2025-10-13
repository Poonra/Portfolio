use clap::{Parser, Subcommand};
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse(); //clap parse argv to our struct

    match cli.cmd {
        Command::Init =>{
            println!("dub init setting up db");
        }
        Command::Asset { action } => match action{

            AssetCmd::Add {symbol, r#type, currency,name} => {
                println!("DUB asset add: {symbol} type= {type} currency={currency} name={:?}", name);
            }
            AssetCmd::List => {
                println!("(stub) asset list: ");
            }
        },

        Command::Txn{action}=> match action{

            TxnCmd::Add { user, symbol, side, qty, price, fee, ts } =>{
                println!("Transaction add user={user} {side} {qty} {symbol} @ {price} fee={fee} ts={ts}");
            }
            TxnCmd::List { user, limit } => {
                println!("(stub) txn list: user={user} limit={limit}");
            }
        },

        Command::Sync { offline } => {
            println!("(stub) sync: offline={offline}");
        }

        Command::Positions { user } => {
            println!("(stub) positions for user={user}");
        }

        Command::Summary { user, json } => {
            if json {
                println!(r#"{{"total_value":0,"total_pl":0,"allocations":[],"user":"{user}"}}"#);
            } else {
                println!("(stub) summary for user={user} -> value=0, P/L=0");
            }
        }
    
    }

    Ok(())
}
