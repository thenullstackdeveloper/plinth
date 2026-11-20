//! `InputSender` that only logs: one line per key event. Lets the panel run on
//! a platform without an input adapter (macOS during development) without
//! sending keys anywhere.

use std::io::Write;
use std::sync::Mutex;

use plinth_application::{InputError, InputSender, KeyEvent, KeyValue};

pub struct LogInputSender<W: Write> {
    sink: Mutex<W>,
}

impl<W: Write> LogInputSender<W> {
    pub fn new(sink: W) -> Self {
        Self {
            sink: Mutex::new(sink),
        }
    }

    pub fn into_inner(self) -> W {
        self.sink.into_inner().unwrap_or_else(|e| e.into_inner())
    }
}

/// A sender that logs to the process's standard error.
pub fn stderr() -> LogInputSender<std::io::Stderr> {
    LogInputSender::new(std::io::stderr())
}

impl<W: Write> InputSender for LogInputSender<W> {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError> {
        let mut sink = self
            .sink
            .lock()
            .map_err(|e| InputError(format!("log sink poisoned: {e}")))?;
        for event in events {
            let value = match event.value {
                KeyValue::Down => "down",
                KeyValue::Up => "up",
            };
            writeln!(sink, "key {} {value}", event.code.0)
                .map_err(|e| InputError(format!("could not write the key log: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::ScanCode;

    #[test]
    fn writes_one_line_per_event() {
        let sender = LogInputSender::new(Vec::new());
        sender
            .emit(&[KeyEvent::down(ScanCode(50)), KeyEvent::up(ScanCode(50))])
            .unwrap();
        assert_eq!(sender.into_inner(), b"key 50 down\nkey 50 up\n".to_vec());
    }
}
