# Plinth — Análisis técnico de la Fase 0

> Análisis previo a implementar, sobre los tres puntos de la Fase 0:
> (1) envío de input, (2) no robar el foco, (3) verificación contra Star Citizen bajo
> Proton. Basado en investigación web + el entorno real detectado.
> **Estado:** análisis. Sin código aún.

---

## 0. Entorno real detectado

| Dato | Valor | Implicación |
|---|---|---|
| Sesión | **Wayland** | Descarta XTEST/xdotool y baja la utilidad de enigo/rdev. |
| Compositor | **Hyprland** (wlroots) | Soporta `nofocus` y layer-shell. La gestión de ventana se hace por reglas del compositor. |
| `/dev/uinput` | **Presente** | Se pueden crear dispositivos virtuales. |
| Grupo `input` | **El usuario pertenece** | uinput accesible **sin root**. Punto de partida ideal. |

> Conclusión de partida: tu máquina es casi el escenario óptimo. uinput es accesible ya,
> y Hyprland tiene las piezas que necesitamos para el panel sin foco.

---

## 1. Veredicto ejecutivo

La Fase 0 es **viable** y el camino correcto está bastante claro:

- **Envío de input → uinput directo** (crate `evdev`, struct `VirtualDevice`). **No** enigo
  ni rdev.
- **Posición/always-on-top/sin-foco → reglas de Hyprland**, no la API de Tauri (que en
  Wayland no controla ni la posición ni el "always on top").
- **EAC → bajo riesgo**: uinput es un HID virtual a nivel de kernel; no toca el proceso
  ni la memoria del juego. El riesgo de EAC son los *hooks* e inyección en proceso, no los
  dispositivos de input externos.
- **El único riesgo real a validar empíricamente** es si Hyprland mantiene el **foco de
  teclado en Star Citizen** mientras tocas el panel en el segundo monitor. Todo lo demás
  es terreno conocido.

---

## 2. Punto 1 — Envío de input

### El eje del problema: X11 vs Wayland

En X11 inyectar teclas era trivial (XTEST/xdotool). En **Wayland es deliberadamente
restringido** por seguridad: un cliente no puede inyectar input a otros. Hay tres vías:

1. **Protocolos Wayland de input virtual** → dependen del compositor; fragmentado.
2. **libei + portales** (xdg-desktop-portal RemoteDesktop) → vía "sancionada" moderna,
   pero pensada para GNOME/KDE (GNOME 46+), aún con bugs, y **no garantiza llegar a un
   juego bajo Proton**.
3. **uinput (módulo del kernel)** → crea un teclado/ratón **virtual a nivel de kernel**,
   *por debajo* del compositor. Funciona igual en X11 y Wayland, y es transparente para
   Hyprland, para Wine/Proton y para EAC porque es indistinguible de un teclado físico.

**uinput es la respuesta** para este proyecto. Es el mismo mecanismo que usan AntiMicroX,
`extest` y prácticamente todas las herramientas de input en Wayland.

### Por qué NO enigo ni rdev (corrección respecto al plan inicial)

- **enigo**: en Linux usa **X11 (xdo) por defecto**; su soporte Wayland/libei es
  experimental, detrás de feature flags, con bugs conocidos (mete caracteres erróneos al
  escribir texto). **No usa uinput.** En Hyprland + Proton no es fiable.
- **rdev**: su `grab` (capturar) usa evdev y funciona en Wayland, pero su `send` (enviar)
  se apoya en X11. No encaja para emitir en Hyprland.

> Por eso el plan inicial ("enigo/rdev como punto de partida") queda **descartado para el
> envío**. La capa de `InputSender` en Linux se construye sobre **uinput directo**.

### Crates recomendados

- **`evdev`** (struct `evdev::uinput::VirtualDevice`): bajo nivel, mantenido, control total.
  **Opción principal.** Escribe a `/dev/uinput`, declara las capacidades (qué teclas
  emite) y emite eventos `EV_KEY` + el `EV_SYN` de sincronización.
- **`mouse-keyboard-input`**: envoltura de más alto nivel sobre uinput; funciona en
  cualquier distro y en X11/Wayland. Útil si se quiere prototipar rápido.

### Permisos y portabilidad

- En tu máquina: **ya cumplido** (grupo `input` + `/dev/uinput`). No hace falta root.
- Para distribuir: incluir una **regla udev** que dé acceso a `/dev/uinput` (o documentar
  que el usuario se añada al grupo `input`).

### Nivel de emulación (nota para la Fase 1)

uinput emite **códigos de tecla de evdev** (`KEY_A`, `KEY_LEFTSHIFT`…), que son
*scancodes* a nivel físico; el compositor aplica el layout (US, ES…) por encima. Star
Citizen vincula por **tecla física**, así que esto encaja. La traducción
*acción de SC → tecla guardada → código `KEY_`* (incluyendo layouts tipo WSAD/ZSQD) es
trabajo del **adaptador de SC en la Fase 1**, no de la Fase 0. La Fase 0 solo necesita:
emitir `KEY_A` y confirmar que llega.

---

## 3. Punto 2 — No robar el foco

### Reformulación del problema con uinput

Con uinput la tecla emitida va **a la ventana que tenga el foco de teclado** (como un
teclado físico). Por tanto la regla es: **el panel NO debe tener el foco de teclado; Star
Citizen sí.** El panel es táctil puro — *nunca* necesita teclado.

**Pieza conceptual clave (a nuestro favor):** Wayland separa el **foco de teclado** del
**foco de puntero/táctil**. Una ventana puede recibir toques (pulsas un botón) **sin**
tener el foco de teclado. Justo lo que queremos.

### Hallazgo crítico: Tauri no gestiona la ventana en Wayland

Confirmado en los issues de Tauri/tao: en Wayland **`always_on_top` no funciona** y
**`set_position` no tiene efecto** (Wayland no tiene coordenadas globales). Es limitación
de Wayland, no un bug puntual.

> Consecuencia de diseño: en Hyprland, la **colocación, el "siempre encima" y el "sin
> foco" del panel se delegan a reglas del compositor** (`windowrulev2`), no al código de
> Tauri. Encaja bien con tu perfil (config declarativa de Hyprland) y se documenta como
> requisito de instalación en Linux.

### Opción A (recomendada para empezar): regla `nofocus` de Hyprland

El panel es una ventana flotante, fijada (`pin`), movida al monitor táctil, con tamaño
fijo y **`nofocus`**. Hyprland tiene exactamente este caso de uso (paneles de
notificación, overlays). Como el panel nunca quiere teclado, encaja perfecto.

- **Rough edge conocido y documentado** (Hyprland issue #5618): `nofocus` puede, en
  multi-monitor, desplazar el foco de *workspace* o llevar el cursor al tocar. Si eso
  hiciera que SC pierda el foco de teclado, las teclas no llegarían. **Este es el riesgo
  nº1 del proyecto y lo que el spike debe validar sí o sí.**

### Opción B (fallback robusto): layer-shell

Renderizar el panel como una **superficie wlr-layer-shell** con
`keyboard_interactivity = none`: por protocolo declara que nunca recibe teclado. Es la
forma "correcta" en Wayland para barras/paneles.

- **Pega:** Tauri/wry usa GTK + `xdg-toplevel`, **no** layer-shell de forma nativa.
  Requeriría puentear `gtk4-layer-shell` sobre wry → más trabajo, más avanzado.
- **Cuándo:** solo si la Opción A no mantiene el foco de teclado en SC de forma fiable.

---

## 4. Punto 3 — Proton + Star Citizen + EAC + verificación

### Cómo recibe Star Citizen el teclado

Wine usa típicamente su **driver X11**, así que en Wayland SC corre **vía XWayland** (el
driver Wayland nativo de Wine aún es experimental). Lo relevante: **el teclado llega a
través del sistema de ventanas y depende del foco** (a diferencia del gamepad, que tiene
sus propias rarezas fuera de foco). Esto **confirma que el foco es el quid**: si SC tiene
el foco de teclado, una tecla de uinput → XWayland → ventana de SC → Wine la traduce a
evento de teclado de Windows. SC la ve como real.

### Hotplug: NO es problema para teclado

Los problemas de "Proton no detecta el dispositivo virtual creado tarde" afectan a
**mandos/joysticks** (que se enumeran vía SDL/evdev al arrancar). El **teclado va por otra
vía**: el compositor detecta teclados nuevos automáticamente y los enruta a la ventana con
foco. Como emulamos **teclas de teclado** (igual que el precedente `streamdeck-starcitizen`,
que solo soporta bindings de teclado), el hotplug no nos afecta.

### Configuración recomendada de Star Citizen para coexistir con el panel

- **Borderless windowed**: `USER.cfg` con `r_WindowMode = 1`. Una ventana normal de
  Hyprland en el monitor principal convive mucho mejor con un panel en el segundo monitor
  que el *fullscreen* exclusivo.
- **Sin gamescope, si es posible.** gamescope es un compositor anidado que **captura el
  input** (`--force-grab-cursor`) y se queda con el teclado; un panel *fuera* de gamescope
  podría no convivir y las teclas de uinput podrían ir a la superficie de gamescope. Usar
  gamescope solo si hace falta para multi-monitor, y entonces **probar explícitamente** la
  convivencia.
- **EAC en Linux**: lo gestiona el `lug-helper` de la comunidad; SC es jugable en Proton.

### EAC y uinput: por qué es bajo riesgo

La guía de EAC en Linux/Proton es explícita: lo peligroso es **inyectar en el proceso del
juego, hookear APIs gráficas o leer/modificar memoria**. uinput **no hace nada de eso**:
crea un HID virtual a nivel de kernel y emite teclas; nunca toca el proceso de SC.
AntiMicroX (basado en uinput) se usa en juegos con EAC bajo Proton. Sumado al precedente de
`streamdeck-starcitizen` y a que SC tolera la emulación de input, la confianza es alta.

> **Reglas innegociables (recordatorio):** solo emitir input vía uinput. Nunca leer/escribir
> memoria ni ficheros del juego. Datos en vivo (futuro) solo desde `Game.log`/APIs oficiales.
> Verificar empíricamente y vigilar bans.

### Plan de verificación del spike (en orden)

1. **uinput aislado:** programa Rust mínimo que cree un `VirtualDevice` y emita `KEY_A`.
   Confirmar el evento con `wev`, `evtest` o `libinput debug-events`.
2. **A una app normal:** foco en un editor de texto → la `a` aparece. Valida la cadena
   uinput → Hyprland → ventana enfocada.
3. **A Star Citizen con foco:** SC en borderless, con foco → emitir una tecla bindada →
   confirmar que la acción ocurre dentro de SC.
4. **EL TEST CRÍTICO (panel sin foco):** ventana de Tauri con regla `nofocus` en el monitor
   táctil → **tocar/clicar** un botón del panel → comprobar con `hyprctl activewindow` que
   **SC sigue teniendo el foco de teclado** → emitir la tecla desde el panel → la acción
   ocurre en SC. Si esto pasa, la Fase 0 está superada.
5. **Sesión real:** jugar un rato usando el panel y comprobar que EAC no marca nada.

Herramientas de inspección útiles: `wev`, `evtest`, `libinput debug-events`,
`hyprctl clients`, `hyprctl activewindow`.

---

## 5. Riesgos, ordenados

| # | Riesgo | Nivel | Mitigación |
|---|---|---|---|
| R1 | `nofocus` no mantiene el foco de teclado en SC en multi-monitor (Hyprland #5618) | **Alto** | Es lo que valida el spike (test 4). Fallback: layer-shell con `keyboard_interactivity=none`. |
| R2 | Gestión de ventana vía reglas de Hyprland no es portable a Windows/macOS | Medio | El MVP es Linux. En Win/Mac se hará con APIs nativas de ventana (futuro). |
| R3 | EAC marca la emulación | Bajo | uinput no toca el proceso; precedentes sólidos. Verificar en sesión real. |
| R4 | gamescope captura el input y rompe la convivencia | Bajo/Medio | Recomendar SC en borderless sin gamescope; si se usa, probar aparte. |
| R5 | Layout de teclado / mapeo acción→`KEY_` | Bajo | Es trabajo del adaptador de SC (Fase 1), no de la Fase 0. |

---

## 6. Decisiones que cierra este análisis

- **Capa de input en Linux = uinput directo** vía crate **`evdev`** (`VirtualDevice`).
  Descartados enigo/rdev para el envío.
- **Gestión de ventana del panel en Hyprland = reglas `windowrulev2`** (`float`, `pin`,
  `monitor`, `size`, `nofocus`), no la API de Tauri.
- **SC en borderless windowed** (`r_WindowMode = 1`), preferiblemente sin gamescope.
- **EAC: vía libre** dentro de las reglas innegociables (solo input, nunca memoria/ficheros).
- El **éxito de la Fase 0** se define por el **test 4** (panel sin foco → tecla llega a SC).

## 7. Lo que queda para confirmar empíricamente (no por web)

1. Que `nofocus` mantenga el foco de teclado en SC al tocar el panel en el 2º monitor (R1).
2. El comportamiento exacto de las reglas de Hyprland para fijar el panel en el monitor táctil.
3. Que EAC no marque nada en una sesión real prolongada.

---

## Fuentes

- [evdev crate — VirtualDevice (uinput)](https://docs.rs/evdev/latest/evdev/uinput/struct.VirtualDevice.html)
- [mouse-keyboard-input (uinput, X11/Wayland)](https://crates.io/crates/mouse-keyboard-input/0.2.0)
- [enigo — backends Linux (X11/Wayland/libei)](https://github.com/enigo-rs/enigo/blob/main/README.md)
- [rdev](https://github.com/Narsil/rdev)
- [AntiMicroX: switch de XTEST a uinput en Wayland (PR #186)](https://github.com/AntiMicroX/antimicrox/pull/186)
- [extest — XTEST reimplementado sobre uinput para Wayland](https://github.com/Supreeeme/extest)
- [Wayland fragmentation / xdotool adventure](https://www.semicomplete.com/blog/xdotool-and-exploring-wayland-fragmentation/)
- [Hyprland Wiki — Window Rules (`nofocus`)](https://wiki.hypr.land/0.48.0/Configuring/Window-Rules/)
- [Hyprland issue #5618 — nofocus y foco entre monitores](https://github.com/hyprwm/Hyprland/issues/5618)
- [Tauri/tao #1134 — always_on_top no funciona en Wayland](https://github.com/tauri-apps/tao/issues/1134)
- [Tauri #3117 — always on top en Wayland](https://github.com/tauri-apps/tauri/issues/3117)
- [Tauri #6394 — asignar ventana a un monitor](https://github.com/tauri-apps/tauri/issues/6394)
- [A Wayland driver for Wine — Collabora](https://www.collabora.com/news-and-blog/news-and-events/a-wayland-driver-for-wine.html)
- [SC LUG knowledge base — Tips and Tricks (borderless, gamescope)](https://github.com/starcitizen-lug/knowledge-base/blob/main/wiki/Tips-and-Tricks.md)
- [Gamescope — ArchWiki](https://wiki.archlinux.org/title/Gamescope)
- [EAC Guidance for Linux and Proton — PulseGeek](https://pulsegeek.com/articles/easy-anti-cheat-guidance-for-linux-and-proton/)
- [streamdeck-starcitizen (precedente: keyboard bindings, sin memoria)](https://github.com/mhwlng/streamdeck-starcitizen)
