use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

const META_PREFIX_TIMEOUT: Duration = Duration::from_millis(30);

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Use a dedicated OS thread instead of tokio::spawn, since
        // crossterm::event::poll() is a blocking call that would tie up
        // a tokio worker thread permanently.
        std::thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        let next_event = if key.code == KeyCode::Esc && key.modifiers.is_empty() {
                            if event::poll(META_PREFIX_TIMEOUT).unwrap_or(false) {
                                event::read().ok()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        for app_event in expand_key_event(key, next_event) {
                            if tx.send(app_event).is_err() {
                                return;
                            }
                        }
                    }
                    Ok(Event::Resize(w, h)) => {
                        if tx.send(AppEvent::Resize(w, h)).is_err() {
                            return;
                        }
                    }
                    _ => {}
                }
            } else if tx.send(AppEvent::Tick).is_err() {
                return;
            }
        });

        Self { rx }
    }

    pub async fn next(&mut self) -> Result<AppEvent> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}

fn expand_key_event(key: KeyEvent, next_event: Option<Event>) -> Vec<AppEvent> {
    if key.code != KeyCode::Esc || !key.modifiers.is_empty() {
        return vec![AppEvent::Key(key)];
    }

    match next_event {
        Some(Event::Key(mut next_key)) if next_key.code != KeyCode::Esc => {
            next_key.modifiers |= KeyModifiers::ALT;
            vec![AppEvent::Key(next_key)]
        }
        Some(Event::Resize(w, h)) => vec![AppEvent::Key(key), AppEvent::Resize(w, h)],
        _ => vec![AppEvent::Key(key)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_key_event_converts_esc_prefixed_key_to_alt() {
        let events = expand_key_event(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            Some(Event::Key(KeyEvent::new(
                KeyCode::Char('1'),
                KeyModifiers::NONE,
            ))),
        );

        assert_eq!(events.len(), 1);
        match events[0] {
            AppEvent::Key(key) => {
                assert_eq!(key.code, KeyCode::Char('1'));
                assert!(key.modifiers.contains(KeyModifiers::ALT));
            }
            _ => panic!("expected key event"),
        }
    }

    #[test]
    fn test_expand_key_event_keeps_bare_escape() {
        let events = expand_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), None);

        assert_eq!(events.len(), 1);
        match events[0] {
            AppEvent::Key(key) => assert_eq!(key.code, KeyCode::Esc),
            _ => panic!("expected key event"),
        }
    }

    #[test]
    fn test_expand_key_event_preserves_resize_after_escape() {
        let events = expand_key_event(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            Some(Event::Resize(120, 40)),
        );

        assert_eq!(events.len(), 2);
        match events[0] {
            AppEvent::Key(key) => assert_eq!(key.code, KeyCode::Esc),
            _ => panic!("expected escape key"),
        }
        match events[1] {
            AppEvent::Resize(w, h) => {
                assert_eq!((w, h), (120, 40));
            }
            _ => panic!("expected resize event"),
        }
    }
}
