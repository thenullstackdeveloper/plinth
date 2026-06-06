#!/usr/bin/env bash
# Architectural fitness test: the core (plinth-domain) and the application layer
# (plinth-application) must NOT depend on any infrastructure crate or on
# game-specific logic.
#
# The strong guarantee comes from the workspace dependency graph (those crates
# aren't even available to domain/application, so a `use` won't compile). This
# script documents that and catches obvious leaks —imports or paths of infra
# crates— without relying on fragile substring matches.

set -euo pipefail

DIRS=(crates/plinth-domain/src crates/plinth-application/src)
# Infrastructure / serialization crates that must not appear in the core.
FORBIDDEN='evdev|tauri|quick_xml|jsonschema|serde|serde_json|serde_yaml_ng|dirs'

status=0

# 1. Imports: `use <crate>...`
if grep -rnE "^[[:space:]]*use[[:space:]]+(${FORBIDDEN})\b" "${DIRS[@]}"; then
  echo "FAIL: the core imports an infrastructure crate (above)." >&2
  status=1
fi

# 2. Qualified paths: `<crate>::...`
if grep -rnE "(^|[^[:alnum:]_])(${FORBIDDEN})::" "${DIRS[@]}"; then
  echo "FAIL: the core uses a path from an infrastructure crate (above)." >&2
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo "PASS: the core is clean of infrastructure dependencies."
fi
exit "$status"
