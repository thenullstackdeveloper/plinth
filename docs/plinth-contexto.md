# Plinth — Documento de contexto

> Documento de traspaso para continuar el análisis y la definición del proyecto.
> Resume las conclusiones y reflexiones de una sesión de brainstorming previa.
> **Estado:** fase de concepción / pre-MVP. Nada de código escrito aún.

-----

## 1. Qué es Plinth

Aplicación de escritorio que convierte una **pantalla táctil pequeña tipo ultrawide (~12”) colocada bajo el monitor** en un panel de control para juegos de PC, al estilo de un Stream Deck pero pensado para gaming.

Funcionalidad núcleo:

- Mostrar **botones con acciones** (pulsar tecla, secuencias, toggles) y **datos en vivo** del juego.
- **Detectar el juego** en ejecución y cargar el panel adecuado, o permitir **selección manual** como alternativa.
- Los paneles (“plantillas”) se definen con un **formato declarativo (YAML)** para que la comunidad pueda crear y compartir los suyos.

### Sobre el nombre

El nombre elegido es **Plinth** (zócalo / basa: la pieza sobre la que se asienta una columna → el panel vive en la base del monitor). Elegante y muy poco usado en software.

Contexto de la investigación de nombres (por si hay que revisitarlo):

- **Zonas quemadas a evitar:** el sufijo `-deck` está saturadísimo (Stream Deck, Deckboard, Macro Deck, OpenDeck, Overdeck, WebDeck, Loupedeck, MATRIC, Monect). “Glass” lo ocupa GameGlass.
- **Descartados por estar ya cogidos:** *Sliver* (framework red-team C2 + software de visualización), *Tessera* (software del procesador LED de Brompton, integra con Stream Deck → demasiado cerca), *Detent* (app de fidget con haptics).
- **Pendiente de verificar por el usuario:** disponibilidad real de dominio (sobre todo `.gg` / `.app` / `.io`) y registro de marca (USPTO / EUIPO) para la clase de software. “Plinth” no mostró conflicto de software de control en las búsquedas, pero la comprobación de dominio y marca queda pendiente.

-----

## 2. Conclusión estratégica (la decisión central)

Tras valorar **monetizar** vs. **liberar gratis**, la conclusión es:

> **Construir Plinth como proyecto personal + pieza de portfolio, gratis para la comunidad. No como producto de pago.**

Razonamiento:

- El usuario es **jugador de Star Citizen** y la app resuelve un *pain* real propio (“scratch your own itch”). Es usuario cero.
- Está en **fase de creación de portfolio**.
- Esta estructura captura valor (uso propio + portfolio) **aunque la adopción sea cero**, y el mantenimiento se justifica porque él mismo la usa.
- Evita la trampa del producto de pago: una suscripción o un producto comercial obligan a soporte continuo que el usuario **no tiene tiempo de sostener**.

Notas sobre monetización (descartada de momento, pero documentada):

- Monetizar este tipo de app es difícil: categoría saturada con alternativas gratis y open source. El foso real no es el código, es el **contenido/comunidad** (caso GameGlass: su valor son miles de “shards”, no su código).
- Lo caro no es construir, es **mantener** (integraciones que se rompen con cada parche del juego), **soporte**, **marketing** y **distribución** → justo lo que choca con “tengo poco tiempo”.
- Si en el futuro se quisiera ingresar algo sin contraer obligaciones: **pago único** o **donaciones** (GitHub Sponsors / Ko-fi) encajan mejor que suscripción.

### Licencia

- Se quiere algo **gratis para uso personal / sin ánimo de lucro**, estilo open source.
- **Matiz importante:** una licencia “solo uso no comercial” **no es técnicamente open source** (la definición OSI no permite restringir el campo de uso); es *source-available*. Es una opción legítima, pero conviene conocer la distinción y que ese tipo de licencias puede frenar a colaboradores.
- **Decisión de licencia concreta: pendiente.** (Candidatos a evaluar: MIT/Apache si se quiere OSS puro; o algo source-available con cláusula no comercial si se quiere reservar la vía comercial; o dual licensing.)

-----

## 3. Decisión de arquitectura clave: plantillas declarativas + comunidad

Separar **configuración (declarativa, YAML)** del **motor**. Esto permite descargar en la comunidad el trabajo de contenido que hundiría a un dev solo.

Ventajas:

- La “tienda” v0 puede ser literalmente **un repositorio de GitHub con Pull Requests**: cero backend.
- Demuestra en el portfolio capacidad de diseñar **arquitectura de extensiones** y de **gestionar comunidad** (vale más en una entrevista que el feature en sí).
- Plantillas como archivos de texto → fáciles de compartir, versionar y revisar.

### Consideraciones de diseño del formato (importantes)

1. **Seguridad (innegociable):** si las plantillas disparan acciones, una plantilla maliciosa de terceros podría hacer daño. Regla de oro: el YAML solo describe acciones de un **catálogo cerrado** (pulsar tecla, secuencia, toggle…), **nunca ejecutar comandos ni código arbitrario**. Validar con un *schema* y rechazar lo que no encaje.
1. **YAML es cómodo de leer pero traicionero de escribir a mano** (indentación). Validar con JSON Schema y dar **errores claros**. TOML es algo más a prueba de errores para no-devs; el público es medio técnico, así que YAML es aceptable, pero es un tradeoff consciente.
1. **Deriva de keybinds:** los atajos cambian entre parches del juego. Incluir metadatos en cada plantilla: `game`, `version`, `author`, etc., para trazabilidad y para avisar de plantillas desactualizadas.
1. **Detección de juego:** detectar por proceso/ventana → cargar la plantilla que coincide. Encaja de forma natural con el formato declarativo.

-----

## 4. Alcance del MVP

Resistir la tentación de construir la plataforma entera. MVP propuesto:

**Incluir:**

- **Un solo juego: Star Citizen** (pain real del usuario, comunidad apasionada, muchísimos keybinds = el problema que de verdad resuelve).
- El **motor** + el **formato declarativo (YAML)** + **1–2 plantillas** que el usuario use de verdad.
- **Selección manual del juego** es suficiente para la v1.

**Aparcar para más adelante (donde mueren los meses):**

- Autodetección del juego (v2 natural).
- Editor visual de plantillas (tipo “Forge” de GameGlass).
- Navegador / marketplace de plantillas in-app.
- App móvil / multi-dispositivo.

-----

## 5. Stack técnico (a decidir)

- Idea inicial del usuario: **Rust + Tauri** (marcado por él como “debatible”).
- Rust es buen diferenciador para portfolio (skills de sistemas + desktop moderno). Pendiente de confirmar/debatir.
- **Decisión de stack: abierta.**

-----

## 6. Panorama competitivo (referencia)

- **GameGlass** (gameglass.gg): el referente más cercano. Modelo freemium + suscripción (Pro ~49,90 USD/año, Lifetime 229 USD) + Lifetime pass. Su valor es la biblioteca de “shards” y la comunidad. Tiene editor visual (“Forge”) y app móvil. Web hecha en Framer.
- **Touch Portal**: companion app freemium (móvil/tablet como deck).
- **Macro Deck**: open source, móvil como macro pad.
- **Elgato Virtual Stream Deck**: deck virtual por software (requiere hardware Elgato para desbloquear).
- **Corsair Xeneon Edge**: hardware de pantalla ultrawide (2560×720, 14,5”) que se monta bajo el monitor → existe el form factor en hardware, pero no es el nicho de software de Plinth.
- **VoiceAttack / remapeadores de joystick**: muy usados en la comunidad de Star Citizen. Señal de que hay **apetito real** por herramientas de automatización de input en ese público.

**Diferenciador de Plinth:** form factor ultrawide-bajo-el-monitor + *game-aware* (detección/carga automática) + plantillas declarativas creadas por la comunidad + gratis.

> ⚠️ **Anti-cheat:** enviar inputs a juegos puede activar sistemas anti-cheat. La comunidad de SC usa estas herramientas habitualmente, pero **verificar la política anti-cheat vigente de Star Citizen** antes de publicitarlo. No darlo por sentado.

-----

## 7. Próximos pasos sugeridos

1. **Diseñar el esquema de la plantilla YAML**: campos de metadatos, tipos de componentes (botón, switch, dial, toggle, etiqueta, widget de datos…) y catálogo cerrado de tipos de acción. Es la decisión de diseño **más costosa de cambiar después**, así que conviene que nazca bien.
1. Definir el **modelo de seguridad** del catálogo de acciones (whitelist + validación por schema).
1. Decidir **stack técnico** (¿Rust + Tauri u otra cosa?).
1. Decidir **licencia**.
1. Esbozar la **arquitectura motor ↔ plantillas ↔ detección de juego ↔ render en pantalla táctil**.
1. Definir cómo se entregan inputs al juego enfocado (capa de envío de teclas/secuencias) y su compatibilidad por SO.