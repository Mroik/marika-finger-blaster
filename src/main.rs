#![allow(clippy::needless_return)]

mod app;
pub mod error;
pub mod event;
pub mod state;

use std::{
    error::Error,
    fs::read_to_string,
    io::{IsTerminal, Read, stdin},
    path::Path,
};

use app::App;
use clap::Parser;
use rand::{Rng, thread_rng};

#[derive(Parser)]
struct Args {
    quote: String,
}

fn generate_quotes(path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
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
async fn main() -> Result<(), Box<dyn Error>> {
    let quote = if !stdin().is_terminal() {
        let mut b = Vec::new();
        stdin().read_to_end(&mut b).unwrap();
        String::from_utf8(b).unwrap()
    } else {
        let args = Args::parse();
        let path = Path::new(&args.quote);
        let mut quotes = generate_quotes(path).unwrap();
        let mut rng = thread_rng();
        let chosen = rng.gen_range(0..quotes.len());
        quotes.remove(chosen)
    };

    // TODO Add more options to choose quotes
    let mut app = App::new(&quote);

    app.start().await?;
    return Ok(());
}
