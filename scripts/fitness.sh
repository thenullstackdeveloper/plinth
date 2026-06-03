#!/usr/bin/env bash
# Fitness test arquitectónico: el núcleo (plinth-domain) y la capa de aplicación
# (plinth-application) NO deben depender de ningún crate de infraestructura ni
# de lógica específica de un juego.
#
# La garantía fuerte la da el grafo de dependencias del workspace (esos crates
# ni siquiera están disponibles para domain/application, así que un `use` no
# compila). Este script lo documenta y caza fugas obvias —imports o rutas de
# crates de infra— sin depender de coincidencias de subcadena frágiles (el
# código y los comentarios están en español: "estar" contiene "star", etc.).

set -euo pipefail

DIRS=(crates/plinth-domain/src crates/plinth-application/src)
# Crates de infraestructura / serialización que no deben aparecer en el núcleo.
FORBIDDEN='evdev|tauri|quick_xml|jsonschema|serde|serde_json|serde_yaml_ng|dirs'

status=0

# 1. Imports: `use <crate>...`
if grep -rnE "^[[:space:]]*use[[:space:]]+(${FORBIDDEN})\b" "${DIRS[@]}"; then
  echo "FALLO: el núcleo importa un crate de infraestructura (arriba)." >&2
  status=1
fi

# 2. Rutas calificadas: `<crate>::...`
if grep -rnE "(^|[^[:alnum:]_])(${FORBIDDEN})::" "${DIRS[@]}"; then
  echo "FALLO: el núcleo usa una ruta de un crate de infraestructura (arriba)." >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "PASS: el núcleo está limpio de dependencias de infraestructura."
fi
exit "$status"
