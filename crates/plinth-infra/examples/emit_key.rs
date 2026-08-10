//! Manual driver for the uinput adapter, used by the Phase 0 spike.
//!
//! Emits one key press after a countdown, so the operator can focus a target
//! window (or click the panel) while it waits.
//!
//! Linux only: uinput is a Linux kernel facility. On other platforms this
//! example compiles but refuses to run, so `cargo build --workspace` stays green
//! on macOS and Windows.
//!
//! Run: cargo run -p plinth-infra --example emit_key -- <keycode> [delay_secs]
//! Example (KEY_L == 38, after 5 s): cargo run -p plinth-infra --example emit_key -- 38 5

#[cfg(target_os = "linux")]
fn main() {
    use std::time::Duration;

    use plinth_application::{InputSender, KeyEvent};
    use plinth_domain::ScanCode;
    use plinth_infra::input_uinput::UinputSender;

    let mut args = std::env::args().skip(1);
    let code: u16 = args
        .next()
        .and_then(|a| a.parse().ok())
        .expect("usage: emit_key <keycode> [delay_secs]");
    let delay: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);

    let sender = UinputSender::open().expect("open /dev/uinput");
    // The compositor needs a moment to notice the new virtual device. Without
    // this, the very first event can be delivered before the device is bound.
    std::thread::sleep(Duration::from_millis(300));

    println!("emitting keycode {code} in {delay}s - focus the target window now");
    std::thread::sleep(Duration::from_secs(delay));

    let key = ScanCode::new(code);
    sender
        .emit(&[KeyEvent::down(key), KeyEvent::up(key)])
        .expect("emit key");
    println!("done");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("emit_key is Linux-only: it drives the uinput adapter.");
    eprintln!("On this platform, use the equivalent InputSender adapter instead.");
    std::process::exit(1);
}
