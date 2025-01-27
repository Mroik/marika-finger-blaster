mod app;
pub mod event;

use std::{error::Error, fs::read_to_string, path::Path};

use app::App;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    quote_folder: String,
}

fn generate_quotes(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut ris = Vec::new();
    if path.is_file() {
        ris.push(
            read_to_string(path)?
                .trim()
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
        );
    } else {
        for f in path.read_dir()? {
            if f.is_err() {
                continue;
            }
            let v = f.unwrap().path();
            if v.is_file() {
                ris.push(
                    read_to_string(v)?
                        .trim()
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect(),
                );
            }
        }
    }
    return Ok(ris);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let path = Path::new(&args.quote_folder);
    let quotes = generate_quotes(&path).unwrap();
    let mut app = App::new(&quotes);

    app.run().await?;
    return Ok(());
}
