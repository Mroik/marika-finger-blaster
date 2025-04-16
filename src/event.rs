use std::{error::Error, time::Duration};

use crossterm::event::{poll, read, KeyCode, KeyModifiers};
use tokio::sync::mpsc::Sender;

use crate::app::TICK_RATE;

#[derive(PartialEq)]
pub enum Event {
    Terminate,
    KeyPress(char),
    Backspace,
    Render,
    ForceRender,
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
            crossterm::event::Event::Resize(_, _) => sender.send(Event::ForceRender).await?,
            crossterm::event::Event::Key(key_event) => {
                match (key_event.code, key_event.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        sender.send(Event::Terminate).await?
                    }
                    (KeyCode::Backspace, _) => sender.send(Event::Backspace).await?,
                    (KeyCode::Char(c), _) => sender.send(Event::KeyPress(c)).await?,
                    _ => (),
                }
            }
            _ => (),
        }
    }
    return Ok(());
}
