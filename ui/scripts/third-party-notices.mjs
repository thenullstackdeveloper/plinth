// Builds THIRD-PARTY-NOTICES.md from the cargo-about JSON of the Windows build
// and the production JavaScript packages. scripts/third-party-notices.sh runs it:
//
//   node ui/scripts/third-party-notices.mjs <cargo-about.json>
//
// The pure functions are exported for the tests; only the program at the bottom
// reads files or runs pnpm.
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const byCodePoint = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const byNameThenVersion = (a, b) =>
  byCodePoint(a.name, b.name) || a.version.localeCompare(b.version, "en", { numeric: true });

/** Every crate cargo-about lists, with its licence expression and the licence texts it uses. */
export function cratesFrom(aboutJson) {
  return aboutJson.crates.map(({ package: { name, version }, license }) => ({
    name,
    version,
    license,
    licenses: aboutJson.licenses
      .filter((l) => l.used_by.some(({ crate }) => crate.name === name && crate.version === version))
      .map(({ id, text }) => ({ id, text }))
      .sort((a, b) => byCodePoint(a.id, b.id)),
  }));
}

/** Every package in the dependency tree of the first project `pnpm list --json` gives, once. */
export function jsPackagesFrom(pnpmListArray) {
  const found = new Map();
  const walk = (dependencies = {}) => {
    for (const [name, { version, path, dependencies: nested }] of Object.entries(dependencies)) {
      const key = `${name}@${version}`;
      if (!found.has(key)) found.set(key, { name, version, path });
      walk(nested);
    }
  };
  walk(pnpmListArray[0].dependencies);
  return [...found.values()].sort(byNameThenVersion);
}

/** A package with the licence of its package.json and the text of each of its LICENSE* files. */
export function jsPackageNotice({ name, version, path }) {
  const { license } = JSON.parse(readFileSync(join(path, "package.json"), "utf8"));
  const files = readdirSync(path, { withFileTypes: true })
    .filter((entry) => entry.isFile() && /^license/i.test(entry.name))
    .map((entry) => entry.name)
    .sort(byCodePoint);
  if (files.length === 0) throw new Error(`${name} ${version} has no LICENSE file in ${path}`);
  return {
    name,
    version,
    license,
    licenses: files.map((file) => ({ id: file, text: readFileSync(join(path, file), "utf8") })),
  };
}

/** The Markdown of THIRD-PARTY-NOTICES.md. */
export function renderNotices({ crates, js }) {
  const sortedCrates = [...crates].sort(byNameThenVersion);
  const sortedJs = [...js].sort(byNameThenVersion);

  const table = (header, entries, source) => [
    `| ${header} | Version | Licence | Source |`,
    "|---|---|---|---|",
    ...entries.map((e) => `| ${e.name} | ${e.version} | ${e.license} | ${source(e)} |`),
  ];

  // One entry per distinct text, numbered by its first user, users in table order.
  // Line endings are normalised so the file has only LF, whatever each licence file uses.
  const texts = new Map();
  for (const entry of [...sortedCrates, ...sortedJs]) {
    for (const text of entry.licenses.map((l) => l.text.replace(/\r\n?/g, "\n"))) {
      const users = texts.get(text) ?? [];
      const user = `${entry.name} ${entry.version}`;
      if (!users.includes(user)) users.push(user);
      texts.set(text, users);
    }
  }

  const lines = [
    "# Third-party notices",
    "",
    "Plinth includes the third-party software listed below. Each entry gives its licence and where",
    "its source code is; the licence texts follow.",
    "",
    "## Rust crates",
    "",
    ...table("Crate", sortedCrates, (c) => `https://crates.io/crates/${c.name}/${c.version}`),
    "",
    "## JavaScript packages",
    "",
    ...table("Package", sortedJs, (p) => `https://www.npmjs.com/package/${p.name}/v/${p.version}`),
    "",
    "## Licence texts",
  ];
  let number = 0;
  for (const [text, users] of texts) {
    number += 1;
    lines.push("", `### Licence text ${number}`, "", `Used by: ${users.join(", ")}`, "", "````text", text, "````");
  }
  return `${lines.join("\n")}\n`;
}

function main([aboutPath]) {
  if (!aboutPath) throw new Error("usage: node ui/scripts/third-party-notices.mjs <cargo-about.json>");
  const ui = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const about = JSON.parse(readFileSync(aboutPath, "utf8"));
  const list = execFileSync("pnpm", ["--dir", ui, "list", "--prod", "--depth", "Infinity", "--json"], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  const js = jsPackagesFrom(JSON.parse(list)).map(jsPackageNotice);
  process.stdout.write(renderNotices({ crates: cratesFrom(about), js }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
