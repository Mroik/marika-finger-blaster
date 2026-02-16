use std::{env, fs, path::PathBuf};

use anyhow::{Result, anyhow};
use rand::{RngExt, rand_core::UnwrapErr, rngs::SysRng};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Quoter {
    #[serde(skip)]
    randomizer: UnwrapErr<SysRng>,
    groups: (
        (usize, usize),
        (usize, usize),
        (usize, usize),
        (usize, usize),
    ),
    quotes: Vec<Quote>,
}

impl Quoter {
    pub fn get_random(&mut self) -> Result<Quote> {
        self.get_range((0, self.quotes.len()))
    }

    fn get_range(&mut self, range: (usize, usize)) -> Result<Quote> {
        if self.quotes.is_empty() {
            return Err(anyhow!("There are no quotes in your quote files"));
        }
        let (l, r) = range;
        if l > r || l > self.quotes.len() || r > self.quotes.len() {
            return Err(anyhow!("Your quotes file is corrupted"));
        }
        Ok(self
            .quotes
            .get(
                self.randomizer
                    .sample(rand::distr::uniform::Uniform::new(l, r + 1)?),
            )
            .cloned()
            .unwrap())
    }

    pub fn get_short(&mut self) -> Result<Quote> {
        self.get_range(self.groups.0)
    }

    pub fn get_medium(&mut self) -> Result<Quote> {
        self.get_range(self.groups.1)
    }

    pub fn get_long(&mut self) -> Result<Quote> {
        self.get_range(self.groups.2)
    }

    pub fn get_huge(&mut self) -> Result<Quote> {
        self.get_range(self.groups.3)
    }
}

#[derive(Deserialize, Clone)]
pub struct Quote {
    pub text: String,
    pub source: Option<String>,
}

pub fn get_config_folder() -> Result<PathBuf> {
    let mut path = match env::var("HOME") {
        Ok(a) => PathBuf::from(a),
        Err(_) => panic!("Can't access config folder"),
    };

    path.push(".config");
    path.push("marika-finger-blaster");
    if !path.exists() {
        if path.is_file() {
            fs::remove_file(&path)?;
        }

        fs::create_dir_all(&path)?;
    }
    Ok(path)
}

pub fn get_quoter() -> Result<Quoter> {
    let mut config_folder = get_config_folder()?;
    config_folder.push("quotes.json");
    if !config_folder.exists() {
        return Err(anyhow!("There's no quotes.json file"));
    }
    let r = fs::File::open(&config_folder)?;
    let quoter = serde_json::from_reader(r)?;
    Ok(quoter)
}
