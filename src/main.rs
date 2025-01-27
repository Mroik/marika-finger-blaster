mod app;
pub mod event;

use std::{error::Error, fs::read_to_string, path::Path};

use app::App;
use clap::Parser;
use rand::{thread_rng, Rng};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    quote_folder: String,
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
    let args = Args::parse();
    let path = Path::new(&args.quote_folder);
    let mut quotes = generate_quotes(&path).unwrap();
    let mut rng = thread_rng();
    let chosen = rng.gen_range(0..quotes.len());
    let quote = quotes.remove(chosen);
    drop(quotes);

    // TODO Add more options to choose quotes
    let mut app = App::new(quote);

    app.run().await?;
    return Ok(());
}
