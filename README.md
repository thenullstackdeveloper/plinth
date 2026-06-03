# Plinth

Panel de control de escritorio para juegos de PC sobre una **pantalla táctil**
(form factor *ultrawide* bajo el monitor). Botones y acciones configurables mediante
**plantillas declarativas**, *game-aware*, y gratis para la comunidad.

> **Estado:** MVP en construcción. El núcleo hexagonal (dominio + casos de uso +
> adaptadores) ya está implementado y testeado; faltan la capa Tauri/React y el
> spike de validación de input (Fase 0) sobre la máquina del desarrollador.

## Idea

Similar en espíritu a un Stream Deck, pero pensado para gaming y deliberadamente
diferenciado: navegación multi-página deslizable estilo tablet, *sidebar* configurable
para diestros/zurdos con candado anti-cambio accidental, y vinculación de las plantillas a
*acciones del juego* (no a teclas físicas), resueltas desde la configuración real de
keybinds. Primer vertical: **Star Citizen**.

## Stack

- **Núcleo:** Rust
- **UI:** Tauri + React + TypeScript (Framer Motion)
- **Envío de input (Linux):** uinput, vía el crate `evdev`

## Arquitectura

Workspace Cargo con **fronteras verificadas por el compilador** (puertos y
adaptadores):

```
plinth-domain        ← (nada)                 puro: lenguaje del producto, agnóstico al juego
plinth-application   → domain                 casos de uso + definición de puertos (traits)
plinth-infra         → application, domain    adaptadores: uinput, keybinds de SC, plantillas, config
plinth-tauri (bin)                            composition root + UI (fase posterior, aún no presente)
```

Un crate de infraestructura (`evdev`, `serde`…) usado desde el núcleo **no
compila**; `scripts/fitness.sh` lo verifica además en CI.

## Desarrollo

```bash
cargo test --workspace --features plinth-application/testing   # tests (CI-safe)
cargo test -p plinth-infra -- --ignored                        # test real de uinput (necesita /dev/uinput)
./scripts/fitness.sh                                           # fitness test arquitectónico
```

## Documentación

- [Contexto del proyecto](docs/plinth-contexto.md)
- [Roadmap](docs/plinth-roadmap.md)
- [Análisis técnico de la Fase 0](docs/plinth-fase0-analisis.md)

## Licencia

Pendiente de decidir. Por ahora, **todos los derechos reservados**.
