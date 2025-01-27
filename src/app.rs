use std::{
    error::Error,
    io::{stdout, Stdout},
};

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::event::{handle_input, Event};

pub const TICK_RATE: u64 = 1000 / 20;

pub struct App {
    stdout: Stdout,
    pub event_tx: Sender<Event>,
    event_rx: Receiver<Event>,
    running: bool,
    quote: Vec<String>,
    current: (usize, usize),
}

impl App {
    pub fn new(quote: String) -> App {
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quote
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            event_rx,
            event_tx,
            running: false,
            current: (0, 0),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.stdout.execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let (ks_tx, ks_rx): (Sender<()>, Receiver<()>) = channel(1);
        let ev = self.event_tx.clone();
        spawn(async {
            start_input_handler(ev, ks_rx).await;
        });

        self.running = true;
        while self.running {
            self.process().await?;
        }
        let _ = ks_tx.send(());

        disable_raw_mode()?;
        self.stdout.execute(LeaveAlternateScreen)?;
        return Ok(());
    }

    async fn process(&mut self) -> Result<(), Box<dyn Error>> {
        match self.event_rx.recv().await.unwrap() {
            Event::Terminate => {
                self.running = false;
            }
            Event::KeyPress(_) => todo!(),
            Event::Backspace => todo!(),
        }
        return Ok(());
    }
}

async fn start_input_handler(ev: Sender<Event>, mut kill_switch: Receiver<()>) {
    loop {
        tokio::select! {
            _ = handle_input(&ev) => (),
            _ = kill_switch.recv() => return,
        }
    }
}
