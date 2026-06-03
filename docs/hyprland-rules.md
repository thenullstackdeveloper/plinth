# Reglas de Hyprland para el panel de Plinth

> En Wayland, Tauri **no** controla la posición, el "siempre encima" ni el "sin
> foco" de la ventana (ver `docs/plinth-fase0-analisis.md` §3). Esas tres cosas
> se delegan a reglas del compositor. Este documento es el requisito de
> instalación del panel en Linux/Hyprland. El adaptador `window_placement` de
> `plinth-infra` es un **no-op** a propósito: la colocación vive aquí.

## 1. Reglas de ventana (`windowrulev2`)

Añade a `~/.config/hypr/hyprland.conf`. Ajusta `title`/`class` al de la ventana
de Plinth y `monitor`/`size`/`move` a tu pantalla táctil.

```ini
# El panel: flotante, fijado, en el monitor táctil, tamaño fijo y SIN foco.
windowrulev2 = float,        title:^(Plinth)$
windowrulev2 = pin,          title:^(Plinth)$
windowrulev2 = nofocus,      title:^(Plinth)$
windowrulev2 = monitor 1,    title:^(Plinth)$
windowrulev2 = size 1920 480,title:^(Plinth)$
windowrulev2 = move 0 600,   title:^(Plinth)$
windowrulev2 = noborder,     title:^(Plinth)$
```

`nofocus` es la pieza clave: el panel recibe toques pero **nunca** el foco de
teclado, de modo que las teclas que inyecta Plinth (vía uinput) van a la ventana
del juego, que conserva el foco.

> **Riesgo nº1 del proyecto (R1):** en multi-monitor, `nofocus` puede mover el
> foco de workspace al tocar (Hyprland issue #5618). **Hay que validarlo
> empíricamente** con el spike de la Fase 0 (test crítico: tocar el panel →
> `hyprctl activewindow` confirma que el juego mantiene el foco de teclado). Si
> falla, el fallback es renderizar el panel como superficie `wlr-layer-shell`
> con `keyboard_interactivity = none` (puente `gtk4-layer-shell` sobre wry).

## 2. Configuración del juego (Star Citizen)

- **Borderless windowed:** en `USER.cfg`, `r_WindowMode = 1`. Una ventana normal
  convive mucho mejor con un panel en el segundo monitor que el *fullscreen*
  exclusivo.
- **Evitar gamescope** si es posible: captura el input y puede quedarse con el
  teclado. Si se usa, probar explícitamente la convivencia con el panel.

## 3. Acceso a `/dev/uinput`

Plinth crea un teclado virtual con uinput. Eso requiere acceso a `/dev/uinput`:

- Pertenecer al grupo `input`, **o**
- instalar la regla udev de `packaging/99-uinput.rules` (recomendado para
  distribución).
