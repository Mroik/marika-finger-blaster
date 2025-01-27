use std::{
    error::Error,
    io::{stdout, Stdout},
    time::Duration,
};

use crossterm::event::{poll, read, KeyCode, KeyModifiers};
use rand::Rng;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::event::Event;

const TICK_RATE: u64 = 1000 / 20;

pub struct App {
    stdout: Stdout,
    quote: Vec<String>,
    event_tx: Sender<Event>,
    event_rx: Receiver<Event>,
    running: bool,
}

impl App {
    pub fn new(quotes: &[Vec<String>]) -> App {
        let mut rng = rand::thread_rng();
        let chosen = rng.gen_range(0..quotes.len());
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quotes[chosen].clone(),
            event_rx,
            event_tx,
            running: false,
        }
    }

    pub async fn run(&mut self) {
        todo!()
    }

    // TODO
    // - [ ] Pause on focus lost
    // - [ ] Invalidate on paste
    async fn handle_input(&mut self) -> Result<(), Box<dyn Error>> {
        while self.running {
            if poll(Duration::from_millis(TICK_RATE))? {
                match read()? {
                    //crossterm::event::Event::FocusGained => todo!(),
                    //crossterm::event::Event::FocusLost => todo!(),
                    //crossterm::event::Event::Paste(_) => todo!(),
                    crossterm::event::Event::Key(key_event) => {
                        if key_event.code == KeyCode::Char('c')
                            && key_event.modifiers == KeyModifiers::CONTROL
                        {
                            self.event_tx.send(Event::Terminate).await?;
                            continue;
                        }
                        if key_event.code == KeyCode::Backspace {
                            self.event_tx.send(Event::Backspace).await?;
                            continue;
                        }
                        if let KeyCode::Char(c) = key_event.code {
                            self.event_tx.send(Event::KeyPress(c)).await?;
                        }
                    }
                    _ => (),
                }
            }
        }
        return Ok(());
    }
}
