# Plinth

Panel de control de escritorio para juegos de PC sobre una **pantalla táctil**
(form factor *ultrawide* bajo el monitor). Botones y acciones configurables mediante
**plantillas declarativas**, *game-aware*, y gratis para la comunidad.

> **Estado:** concepción / pre-MVP. Fase de análisis y diseño — todavía sin código.

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

## Documentación

- [Contexto del proyecto](docs/plinth-contexto.md)
- [Roadmap](docs/plinth-roadmap.md)
- [Análisis técnico de la Fase 0](docs/plinth-fase0-analisis.md)

## Licencia

Pendiente de decidir. Por ahora, **todos los derechos reservados**.
