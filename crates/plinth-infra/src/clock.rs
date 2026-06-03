//! Reloj real del sistema.

use std::time::Duration;

use plinth_application::Clock;

#[derive(Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn sleep(&self, dur: Duration) {
        std::thread::sleep(dur);
    }
}
