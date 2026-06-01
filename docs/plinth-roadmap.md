# Plinth — Roadmap

> Documento de hoja de ruta. Recoge las decisiones tomadas y el plan de construcción
> tras las sesiones de análisis. **Estado:** fase de concepción / pre-MVP.
> Los detalles técnicos finos (interfaces, crates, esquema concreto) se cierran en el
> análisis previo a cada fase de implementación, no aquí.

---

## 1. Visión

Aplicación de escritorio que convierte una **pantalla táctil pequeña tipo ultrawide
(~12") bajo el monitor** en un panel de control para juegos de PC. Similar en espíritu
a un Stream Deck, pero **pensado para gaming y deliberadamente diferenciado** (no es un
clon).

Diferenciadores frente a la competencia (GameGlass, Touch Portal, Macro Deck…):

- Form factor **ultrawide bajo el monitor**.
- **Game-aware**: las plantillas se vinculan a *acciones del juego*, no a teclas físicas.
- Navegación **multi-página estilo tablet** (swipe), no rejilla fija + carpetas.
- **Sidebar configurable** para diestros/zurdos, con **candado** anti-cambio accidental.
- Plantillas **declarativas creadas por la comunidad**.
- **Gratis**.

---

## 2. Decisiones tomadas

| Tema | Decisión |
|---|---|
| **Modelo** | Proyecto personal + pieza de portfolio. Gratis para la comunidad. No producto de pago. |
| **Juego del MVP** | Star Citizen (pain real del usuario, comunidad apasionada, muchísimos keybinds). |
| **Stack** | **Rust + Tauri**. Frontend en **React + TypeScript** con **Framer Motion** para animaciones. Núcleo de sistemas en **Rust**. |
| **Formato de plantillas** | **YAML** como formato de autoría, **con un JSON Schema como contrato/fuente de verdad**. |
| **Detección de juego (MVP)** | **Selección manual**. La autodetección se aparca. |
| **Seguridad** | El YAML solo describe acciones de un **catálogo cerrado**. Nunca ejecutar comandos ni código arbitrario. Nunca tocar ficheros ni memoria del juego. |
| **"Tienda" v0** | Repositorio de GitHub con Pull Requests. Cero backend. |

### Por qué este stack (resumen del razonamiento)

- El frontend en React/TS aprovecha la fuerza del desarrollador (fullstack/mobile, TS,
  React Native, Vue). El núcleo en Rust profundiza la skill *rara y diferenciadora*
  para ese perfil (sistemas).
- Un proyecto Rust+Tauri **ambicioso de verdad** (input multiplataforma, gestión de foco)
  cuenta una historia de **profundidad**, no de repetición.
- React abre la puerta natural a una futura versión móvil (React → React Native).

---

## 3. Anti-cheat — validado (clave para que el proyecto sea viable)

Star Citizen usa **Easy Anti-Cheat (EAC)**. La línea es nítida:

- ✅ **Permitido**: emulación de entrada externa (enviar teclas). Es lo que hacen
  VoiceAttack, Joystick Gremlin (listado compatible), remapeadores y un plugin de
  Stream Deck para SC que existió durante años con esta misma arquitectura.
- ❌ **Prohibido (ban / bloqueo de login)**: alterar ficheros del juego o leer/editar
  su memoria.

**Reglas de oro innegociables:**

1. Plinth **solo inyecta input**. Nunca toca ficheros ni memoria del juego.
2. Si en el futuro se muestran "datos en vivo", **solo** desde fuentes seguras
   (ej. el propio `Game.log` del juego, o APIs oficiales). **Nunca** lectura de memoria.

> El spike técnico inicial confirmará en la práctica, sobre la máquina del desarrollador
> (Linux + Proton), que el envío de teclas llega a SC sin incidencias.

---

## 4. Alcance del MVP

> **Encuadre.** Star Citizen **no es el núcleo**: el núcleo es la base de escritorio
> agnóstica al juego. SC es el **primer vertical** y se construye primero porque su
> adaptador se apoya sobre ese núcleo genérico y **reaprovecha todo el trabajo** — ir
> SC-primero no desperdicia nada. Que el núcleo siga limpio de lógica específica de SC se
> garantiza por **diseño de interfaces**, no por el orden de construcción.
>
> **Centro de gravedad.** El contenido es barato: una vez hecha la lógica, añadir un
> botón o una plantilla es trivial. El riesgo y el valor reales se concentran en la
> **capa de input**: que la pulsación llegue al juego bajo **Proton/Wine** y que el panel
> táctil **no robe el foco** al juego. Ahí es donde se gana o se pierde el proyecto.

**Incluido:**

- Motor + formato declarativo (YAML/Schema) + capa de envío de input.
- **Un solo juego: Star Citizen.**
- **Vinculación semántica**: las plantillas referencian *acciones* de SC; el motor las
  resuelve a teclas leyendo la configuración real de keybinds del juego (defaults +
  overrides del usuario). Esto neutraliza el problema de la deriva de keybinds entre
  parches.
- **UI multi-página** deslizable (swipe izq/dcha) con animación agradable.
- **Sidebar configurable** izquierda/derecha (diestros/zurdos).
- **Candado** en la sidebar que bloquea el cambio de página (caso: asignar escudos en
  combate sin cambiar de pantalla por accidente).
- **Selección manual** del juego.
- **1–2 plantillas reales** de SC que el desarrollador use de verdad.
- Ventana sin bordes, siempre encima, en el display táctil, **sin robar el foco** al juego.

**Detección de SO** (Windows/Linux/macOS): el motor detecta el sistema y elige la
implementación de envío de input correspondiente. Para el MVP se prioriza **Linux**
(donde el desarrollador hace dogfooding); las otras plataformas quedan como ampliación.

---

## 5. Arquitectura base (alto nivel)

Principio rector: **el núcleo es independiente del framework de UI.** Si mañana cambiara
la capa visual, el motor sigue en pie.

```
UI / render (React + Framer Motion, dentro de Tauri)
   · swipe entre páginas · sidebar · candado · animaciones
   · NO envía teclas: solo describe intención
        │  (comandos Tauri / invoke)
        ▼
NÚCLEO (Rust)
   · Validación de plantilla contra JSON Schema (+ errores claros)
   · Catálogo cerrado de acciones  ← frontera de seguridad
   · Adaptador por-juego (SC: acción → tecla, desde el keybind real)
   · Despacho de input por SO (Win / Linux / macOS)
   · Runtime: página actual, estado del candado, toggles, gestión de foco
```

**Dos categorías de acción, separadas por diseño:**

- **Acciones de juego** → envían teclas al juego (catálogo cerrado, pasan por la capa de
  input del SO).
- **Acciones de app** → navegar de página, alternar el candado, cambiar la sidebar de
  lado. **Nunca tocan el juego**; viven dentro del motor y son seguras por definición.

**Sobre YAML:** se mantiene como formato de autoría por ser legible, admitir comentarios
y producir *diffs* limpios en los Pull Requests (la tienda v0). Pero la decisión que de
verdad importa es el **JSON Schema versionado** como contrato; el formato de superficie
es casi un detalle. Como las plantillas de SC se pueden *generar* desde el export de
keybinds, el riesgo de escribirlas a mano pesa poco.

---

## 6. Plan de construcción (de dentro hacia fuera)

El orden prioriza retirar riesgo, no construir lo vistoso primero.

### Fase 0 — Spike de validación
- Envío de input en Rust sobre Linux: que una tecla y una secuencia lleguen a Star
  Citizen bajo Proton **sin robar el foco**.
- Es el mayor riesgo técnico y el diferenciador del proyecto, y donde se concentra el
  trabajo difícil (Proton/Wine + gestión de foco). Todo lo demás —botones, plantillas,
  páginas— es trivial en comparación.
- Si sale verde, el resto es ejecución sobre terreno conocido.

### Fase 1 — Núcleo / motor
- JSON Schema de la plantilla (metadata + páginas + sidebar + acciones).
- Catálogo cerrado de acciones (juego + app) y validación con errores claros.
- Adaptador de Star Citizen: resolver acción → tecla desde el keybind real del juego.
- Capa de envío de input (al menos Linux para el MVP).

### Fase 2 — UI / panel
- Render de la plantilla en React.
- Multi-página con swipe (Framer Motion).
- Sidebar configurable izquierda/derecha.
- Candado que desactiva el swipe.
- Ventana sin bordes / siempre encima / sin robar foco / en el display táctil.
- Desarrollo con **ratón** mientras no se tenga la pantalla táctil (suficiente para todo
  el desarrollo).

### Fase 3 — Plantillas reales + uso
- 1–2 plantillas de SC de uso real (dogfooding).
- Selección manual de juego.

### Fase 4 — Pulido + publicación
- Empaquetado, documentación, licencia.
- Repositorio público como "tienda v0" (plantillas vía Pull Request).

---

## 7. Aparcado / futuro (post-MVP)

> Donde mueren los meses. Se documenta para no perderlo, pero **no** es MVP.

- **Autodetección del juego** (por proceso/ventana → carga la plantilla que coincide).
- **Editor visual de plantillas** (estilo "Forge" de GameGlass).
- **Navegador / marketplace** de plantillas in-app.
- **Datos en vivo** del juego — solo desde fuentes seguras (`Game.log`, APIs oficiales).
- **Soporte Windows y macOS** de la capa de input (el MVP prioriza Linux).
- **Arquitectura cliente-servidor (punto de fantasía):** servidor en el PC + app
  móvil/tablet que reutiliza la UI de React (→ React Native) y envía las acciones al
  servidor, que lanza la pulsación en el PC. La arquitectura actual ya lo permitiría
  (la UI solo envía intención; esa frontera podría volverse de red). **No es prioridad:**
  la prioridad es el PC con pantalla táctil.

---

## 8. Decisiones pendientes

- **Licencia. → DIFERIDA, ABIERTA A DEBATE.** Decisión pospuesta a propósito; mientras el
  repo sea privado (y aun público sin licencia), rige *todos los derechos reservados*, así
  que el trabajo ya está protegido por defecto. El usuario ha matizado su intención:
  *"gratis para la comunidad, pero que nadie saque dinero de mi trabajo"*. Eso reabre el
  debate entre dos caminos:
  - **Source-available no comercial** (ej. PolyForm Noncommercial): refleja literalmente
    ese deseo (uso libre no comercial, sin que terceros lucren). **Contra:** no es "open
    source" según la OSI, puede frenar a contribuidores y resta algo de alcance como pieza
    de portfolio.
  - **Permisiva** (Apache-2.0 / MIT): máxima lectura y reutilización del código → mejor
    señal de portfolio. **Contra:** permite explícitamente el uso comercial por terceros,
    que es justo lo que el usuario quiere evitar.
  - *Punto a resolver en el debate:* cuánto pesa "que nadie lucre" frente al alcance de
    portfolio. Decidir **antes de hacer público** el repo (revertir es difícil si la
    comunidad ya usa el código bajo una licencia permisiva).
- **SO objetivo de v1.** Recomendación: **Linux primero** (donde se hace dogfooding).
- **Dominio y marca.** Verificar disponibilidad de `plinth` (`.gg` / `.app` / `.io`) y
  registro de marca (USPTO / EUIPO) para software. Sin conflicto aparente en software de
  control, pero la comprobación queda pendiente.

---

## 9. Referencia competitiva (breve)

- **GameGlass** (gameglass.gg): el referente más cercano. Freemium + suscripción. Su
  valor es la biblioteca de "shards" y la comunidad, no el código. Tiene editor visual y
  app móvil.
- **Touch Portal / Macro Deck / Elgato Virtual Stream Deck**: variantes de deck por
  software.
- **Corsair Xeneon Edge**: hardware de pantalla ultrawide bajo el monitor → el form
  factor existe en hardware.
- **VoiceAttack / remapeadores**: muy usados en la comunidad de SC → señal de apetito
  real por herramientas de input en ese público.
