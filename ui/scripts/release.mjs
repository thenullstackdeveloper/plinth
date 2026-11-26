// Builds the installer with one command from anywhere: `pnpm --dir ui release`.
// The Tauri CLI only finds the app when it runs from crates/plinth-tauri, so the
// working directory is set here, from this file's location, and every argument
// is passed through to `tauri build` (for example `--no-bundle`).
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ui = join(dirname(fileURLToPath(import.meta.url)), "..");

const { status } = spawnSync("tauri", ["build", ...process.argv.slice(2)], {
  cwd: join(ui, "..", "crates", "plinth-tauri"),
  stdio: "inherit",
  shell: true,
});

process.exit(status ?? 1);
