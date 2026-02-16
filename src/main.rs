#![allow(clippy::needless_return)]

mod app;
pub mod config;
pub mod error;
pub mod input;
pub mod state;

use std::{
    fs::read_to_string,
    io::{IsTerminal, Read, stdin},
    path::Path,
};

use anyhow::Result;
use app::App;
use clap::Parser;
use rand::Rng;

use crate::config::{Quote, get_quoter};

#[derive(Parser)]
struct Args {
    /// Turns all text into lowercase (NOOB mode)
    #[arg(short, long)]
    lower: bool,
    #[arg(short, long)]
    short: bool,
    #[arg(short, long)]
    medium: bool,
    #[arg(short, long)]
    long: bool,
    #[arg(short, long)]
    huge: bool,
    quote: Option<String>,
}

fn generate_quotes(path: &Path) -> Result<Vec<String>> {
    let mut ris = Vec::new();
    if path.is_file() {
        ris.push(read_to_string(path)?);
    } else {
        for f in path.read_dir()? {
            if f.is_err() {
                continue;
            }
            let v = f.unwrap().path();
            if v.is_file() {
                ris.push(read_to_string(v)?);
            }
        }
    }
    return Ok(ris);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut quote = if !stdin().is_terminal() {
        let mut b = Vec::new();
        stdin().read_to_end(&mut b).unwrap();
        Quote {
            text: String::from_utf8(b)?,
            source: None,
        }
    } else if let Some(q) = &args.quote {
        let path = Path::new(q);
        let mut quotes = generate_quotes(path).unwrap();
        let mut rng = rand::rand_core::UnwrapErr(rand::rngs::SysRng::default());
        let chosen = rng.next_u64() as usize;
        Quote {
            text: quotes.remove(chosen),
            source: None,
        }
    } else {
        let mut specifier = 0;
        if args.short {
            specifier += 1;
        }
        if args.medium {
            specifier += 1;
        }
        if args.long {
            specifier += 1;
        }
        if args.huge {
            specifier += 1;
        }
        if specifier > 1 {
            panic!("You can't use more than one quote length specifier");
        }
        let mut quoter = get_quoter()?;
        if args.short {
            quoter.get_short()?
        } else if args.medium {
            quoter.get_medium()?
        } else if args.long {
            quoter.get_long()?
        } else if args.huge {
            quoter.get_huge()?
        } else {
            quoter.get_random()?
        }
    };

    if args.lower {
        quote.text = quote.text.to_lowercase();
    }

    // TODO Add more options to choose quotes
    let mut app = App::new(&quote);

    app.start().await?;
    return Ok(());
}
