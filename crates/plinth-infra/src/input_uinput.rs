//! `InputSender` sobre **uinput** (crate `evdev`). Crea un teclado virtual a
//! nivel de kernel, *por debajo* del compositor, así que funciona igual en
//! X11/Wayland y es transparente para Hyprland, Wine/Proton y EAC (ver
//! `docs/plinth-fase0-analisis.md` §2).
//!
//! `VirtualDevice::emit` requiere `&mut self` y la traza `InputSender::emit` es
//! `&self`, así que el dispositivo vive tras un `Mutex` (además permite
//! compartir el sender entre hilos, p. ej. los comandos de Tauri).

use std::sync::Mutex;

use evdev::{uinput::VirtualDevice, AttributeSet, InputEvent, KeyCode, KeyEvent as EvKeyEvent};
use plinth_application::{InputError, InputSender, KeyEvent, KeyValue};

pub struct UinputSender {
    device: Mutex<VirtualDevice>,
}

impl UinputSender {
    /// Abre `/dev/uinput` y declara un teclado virtual capaz de emitir todo el
    /// rango de teclas estándar. Requiere acceso a `/dev/uinput` (grupo
    /// `input` o la regla udev de `packaging/99-uinput.rules`).
    pub fn open() -> Result<Self, InputError> {
        let mut keys = AttributeSet::<KeyCode>::new();
        // Declara el rango de teclas estándar de teclado (1..=248 cubre letras,
        // dígitos, función, navegación y numpad).
        for code in 1u16..=248 {
            keys.insert(KeyCode::new(code));
        }

        let device = VirtualDevice::builder()
            .map_err(|e| InputError(format!("no se pudo abrir /dev/uinput: {e}")))?
            .name("Plinth Virtual Keyboard")
            .with_keys(&keys)
            .map_err(|e| InputError(format!("no se pudieron declarar las teclas: {e}")))?
            .build()
            .map_err(|e| InputError(format!("no se pudo crear el dispositivo virtual: {e}")))?;

        Ok(Self {
            device: Mutex::new(device),
        })
    }
}

fn to_evdev_event(event: &KeyEvent) -> InputEvent {
    let value = match event.value {
        KeyValue::Down => 1,
        KeyValue::Up => 0,
    };
    EvKeyEvent::new(KeyCode::new(event.code.get()), value).into()
}

impl InputSender for UinputSender {
    fn emit(&self, events: &[KeyEvent]) -> Result<(), InputError> {
        let raw: Vec<InputEvent> = events.iter().map(to_evdev_event).collect();
        let mut device = self
            .device
            .lock()
            .map_err(|_| InputError("mutex del dispositivo envenenado".into()))?;
        device
            .emit(&raw)
            .map_err(|e| InputError(format!("fallo emitiendo eventos: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plinth_domain::ScanCode;

    /// Test de integración real: crea un `VirtualDevice` y emite `KEY_A`.
    /// Requiere `/dev/uinput` accesible → `#[ignore]` para que no corra en CI.
    /// Correr en la máquina del dev con: `cargo test -p plinth-infra -- --ignored`
    /// y verificar con `wev` / `evtest` / `libinput debug-events`.
    #[test]
    #[ignore = "requiere /dev/uinput (máquina del dev, no CI)"]
    fn emits_key_a_to_real_uinput() {
        let sender = UinputSender::open().expect("abrir uinput");
        let key_a = ScanCode::new(30);
        sender
            .emit(&[KeyEvent::down(key_a), KeyEvent::up(key_a)])
            .expect("emitir KEY_A");
    }
}
