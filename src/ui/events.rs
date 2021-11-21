use super::*;

pub struct UseKeyInput;

impl UseKeyInput {
    pub fn new(tick_rate: Duration, sender: Sender<AppEvent>) {
        std::thread::spawn(move || {
            loop {
                if crossterm::event::poll(tick_rate).unwrap() {
                    if let event::Event::Key(key) = event::read().unwrap() {
                        sender.send(AppEvent::Key(key)).unwrap();
                    }
                }
                // sender.send(AppEvent::Tick).unwrap();
            }
        });
    }
}
