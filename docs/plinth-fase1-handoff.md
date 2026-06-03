# Plinth — Handoff de implementación (Fase 1: núcleo hexagonal)

> Estado de traspaso al cerrar la sesión que implementó el núcleo. Léelo si
> retomas el trabajo sin el contexto de la conversación original.

## Qué se construyó (alcance de este trabajo)

Núcleo hexagonal del MVP, en su parte **verificable en CI sin hardware**:

- **`crates/plinth-domain`** — puro, agnóstico al juego. `Action` (catálogo
  cerrado, **sin** variante `Raw` → comando arbitrario irrepresentable),
  `Template` (raíz de agregado inmutable con invariantes), `RuntimeState`
  (única entidad mutable), `ScanCode`/`KeySequence`/`KeyStep`, `ActionCatalog`
  (autoridad de legalidad). Sin dependencias de infraestructura. **13 tests**.
- **`crates/plinth-application`** — casos de uso (`execute_action`,
  `query_render_state`, `navigate`, `go_to_page`, `toggle_lock`,
  `switch_sidebar_side`, `load_template`, `refresh_keybinds`), definición de
  **puertos** (`InputSender`, `GameKeybindProvider`, `TemplateSource`, `Clock`,
  `ConfigStore`, `WindowPlacement`), DTOs (`RenderState`) y **fakes**
  reutilizables tras la feature `testing`. **9 tests**.
- **`crates/plinth-infra`** — adaptadores: `input_uinput` (evdev/uinput),
  `keybinds_sc` (XML exportado de SC), `template_fs` (YAML + JSON Schema +
  mapper), `config_fs` (XDG), `clock`, `window_placement` (no-op en Linux).
  **7 tests + 1 `#[ignore]` (uinput real) + 2 de integración (smoke)**.
- **No-crate:** `schema/plinth.template.schema.json`, `docs/hyprland-rules.md`,
  `packaging/99-uinput.rules`, `.github/workflows/ci.yml`, `scripts/fitness.sh`.

## Decisiones de refinement sobre el plan original

1. **`EvdevKeyCode` → `ScanCode`.** El nombre original metía un subsistema de
   Linux en el dominio agnóstico. `ScanCode` mantiene la frontera limpia también
   en el nombre (los valores siguen siendo códigos de evdev por ahora).
2. **Fitness test por imports, no por subcadena.** El grep
   `star|evdev|tauri|…` daba falsos positivos en español (`estar` contiene
   `star`). `scripts/fitness.sh` comprueba `use`/rutas de crates de infra en
   `plinth-domain` y `plinth-application` — la frontera real y no frágil.
3. **`InputSender::emit(eventos)` + `Clock` en un `SequenceRunner`** (en lugar
   de `send(KeySequence)`). Así el timing de los pasos `Hold` es testeable con
   `FakeClock`.
4. **`TemplateSource::load → Template`** ya validado: el `serde`/parseo
   (`RawTemplateDoc`) queda interno a infra, sin fugar a application.
5. **`keybinds_sc` — limitación conocida.** Soporta bindings de **teclado de
   una sola tecla** (`kb1_<tecla>`). Los acordes con modificadores
   (`kb1_lshift+t`) y joystick/ratón (`js1_*`, `mo1_*`) se registran en el
   catálogo (acción "conocida") pero **no** producen binding emitible → la
   resolución da `NotBound`. El modelo `KeySequence` no expresa pulsaciones
   simultáneas; soportar acordes es una mejora posterior.

## Verificación (reproducible)

```bash
cargo test --workspace --features plinth-application/testing   # 31 passed, 1 ignored
cargo clippy --workspace --all-targets --features plinth-application/testing -- -D warnings
cargo fmt --all --check
./scripts/fitness.sh                                           # PASS: núcleo limpio
cargo test -p plinth-infra -- --ignored                        # test real de uinput (necesita /dev/uinput)
```

## Pendiente (fases siguientes, requieren la máquina del dev)

- **Fase 0 (spike de input):** validar empíricamente que `nofocus` de Hyprland
  mantiene el foco de teclado en SC al tocar el panel (riesgo R1). El "seed" ya
  está en `plinth-infra::input_uinput` (`UinputSender::open` + `emit`); correr el
  test `#[ignore]` y verificar con `wev`/`evtest`. Ver `docs/hyprland-rules.md`.
- **`plinth-tauri` (bin):** composition root + comandos `#[tauri::command]`
  finos (deserializan → caso de uso → serializan). Cablea los adaptadores
  concretos en los casos de uso.
- **Frontend React+TS:** render desde `RenderState`, swipe multi-página
  (Framer Motion), sidebar izq/dcha, candado que desactiva el swipe.
- **Plantillas reales de SC** (YAML) + dogfooding.
- **Mejora:** soporte de acordes en `keybinds_sc` (requiere extender el modelo
  de secuencia del dominio para expresar pulsaciones simultáneas).

## Mapa rápido para orientarse

- Lógica de despacho (corazón): `crates/plinth-application/src/usecases.rs`
  (`execute_action` + `run_sequence`).
- Frontera de seguridad: `crates/plinth-domain/src/catalog.rs`
  (`ActionCatalog::resolve`) y `action.rs` (sin `Raw`).
- Adaptador SC: `crates/plinth-infra/src/keybinds_sc.rs`.
- Contrato de plantilla: `schema/plinth.template.schema.json` +
  `crates/plinth-infra/src/template_fs.rs`.
