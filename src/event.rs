use std::{error::Error, time::Duration};

use crossterm::event::{poll, read, KeyCode, KeyModifiers};
use tokio::sync::mpsc::Sender;

use crate::app::TICK_RATE;

pub enum Event {
    Terminate,
    KeyPress(char),
    Backspace,
}

// TODO
// - [ ] Pause on focus lost
// - [ ] Invalidate on paste
pub async fn handle_input(sender: &Sender<Event>) -> Result<(), Box<dyn Error>> {
    if poll(Duration::from_millis(TICK_RATE))? {
        match read()? {
            //crossterm::event::Event::FocusGained => todo!(),
            //crossterm::event::Event::FocusLost => todo!(),
            //crossterm::event::Event::Paste(_) => todo!(),
            crossterm::event::Event::Key(key_event) => {
                if key_event.code == KeyCode::Char('c')
                    && key_event.modifiers == KeyModifiers::CONTROL
                {
                    sender.send(Event::Terminate).await?;
                    return Ok(());
                }
                if key_event.code == KeyCode::Backspace {
                    sender.send(Event::Backspace).await?;
                    return Ok(());
                }
                if let KeyCode::Char(c) = key_event.code {
                    sender.send(Event::KeyPress(c)).await?;
                }
            }
            _ => (),
        }
    }
    return Ok(());
}
