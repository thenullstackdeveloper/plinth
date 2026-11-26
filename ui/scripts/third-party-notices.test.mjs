// @vitest-environment node
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

import { cratesFrom, jsPackageNotice, jsPackagesFrom, renderNotices } from "./third-party-notices.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..");
const fixture = (name) => JSON.parse(readFileSync(join(here, "fixtures", name), "utf8"));

const crate = (name, version, text) => ({
  name,
  version,
  license: "MIT",
  licenses: [{ id: "MIT", text }],
});

const occurrences = (haystack, needle) => haystack.split(needle).length - 1;

const tableRows = (notices, heading) => {
  const start = notices.indexOf(`## ${heading}\n`);
  const end = notices.indexOf("\n## ", start + 1);
  return notices
    .slice(start, end)
    .split("\n")
    .filter((line) => line.startsWith("| ") && !line.startsWith("| Crate |") && !line.startsWith("| Package |"));
};

describe("third-party notices", () => {
  const temporary = [];

  afterEach(() => {
    for (const dir of temporary.splice(0)) rmSync(dir, { recursive: true, force: true });
  });

  it("turns the cargo-about json into crates with every licence they use", () => {
    const crates = cratesFrom(fixture("cargo-about.json"));

    expect(crates.map((c) => `${c.name} ${c.version}`).sort()).toEqual([
      "dpi 0.1.2",
      "dtoa-short 0.3.5",
      "tao 0.37.3",
      "webview2-com 0.39.1",
    ]);
    const dpi = crates.find((c) => c.name === "dpi");
    expect(dpi.license).toBe("Apache-2.0 AND MIT");
    expect(dpi.licenses.map((l) => l.id)).toEqual(["Apache-2.0", "MIT"]);
    expect(dpi.licenses[0].text).toContain("Apache License");
    expect(dpi.licenses[1].text).toContain("Permission is hereby granted");
  });

  it("renders every crate with its version, licence and source link", () => {
    const crates = cratesFrom(fixture("cargo-about.json"));
    const notices = renderNotices({ crates, js: [] });

    expect(crates).toHaveLength(4);
    for (const c of crates) {
      expect(notices).toContain(
        `| ${c.name} | ${c.version} | ${c.license} | https://crates.io/crates/${c.name}/${c.version} |\n`,
      );
    }
  });

  it("prints each licence text once, with everyone who uses it", () => {
    const about = fixture("cargo-about.json");
    const apache = about.licenses.find((l) => l.id === "Apache-2.0").text;
    const notices = renderNotices({ crates: cratesFrom(about), js: [] });

    expect(occurrences(notices, apache)).toBe(1);
    expect(occurrences(notices, "### Licence text ")).toBe(3);
    expect(notices).toContain(`Used by: dpi 0.1.2, tao 0.37.3\n\n\`\`\`\`text\n${apache}\n\`\`\`\``);
    expect(notices).toContain("Used by: dpi 0.1.2, webview2-com 0.39.1\n");
    expect(notices).toContain("Used by: dtoa-short 0.3.5\n");
    expect(notices.endsWith("````\n")).toBe(true);
    expect(notices.endsWith("\n\n")).toBe(false);
  });

  it("renders JavaScript packages with their licence texts", () => {
    const makePackage = (name, license, files) => {
      const dir = mkdtempSync(join(tmpdir(), "notices-"));
      temporary.push(dir);
      writeFileSync(join(dir, "package.json"), JSON.stringify({ name, version: "1.0.0", license }));
      writeFileSync(join(dir, "README.md"), "not a licence");
      mkdirSync(join(dir, "dist"));
      for (const [file, text] of Object.entries(files)) writeFileSync(join(dir, file), text);
      return jsPackageNotice({ name, version: "1.0.0", path: dir });
    };
    const dual = makePackage("dual", "Apache-2.0 OR MIT", {
      "LICENSE-MIT": "MIT text of dual",
      "LICENSE-APACHE-2.0": "Apache text of dual",
    });
    const single = makePackage("single", "0BSD", { "LICENSE.txt": "0BSD text\r\nof single" });

    expect(dual.licenses.map((l) => l.id)).toEqual(["LICENSE-APACHE-2.0", "LICENSE-MIT"]);
    const notices = renderNotices({ crates: [], js: [single, dual] });
    expect(notices).toContain(
      "| dual | 1.0.0 | Apache-2.0 OR MIT | https://www.npmjs.com/package/dual/v/1.0.0 |\n",
    );
    expect(notices).toContain("| single | 1.0.0 | 0BSD | https://www.npmjs.com/package/single/v/1.0.0 |\n");
    expect(notices).toContain("Used by: dual 1.0.0\n\n````text\nApache text of dual\n````");
    expect(notices).toContain("Used by: dual 1.0.0\n\n````text\nMIT text of dual\n````");
    expect(notices).toContain("Used by: single 1.0.0\n\n````text\n0BSD text\nof single\n````");
    expect(notices).not.toContain("\r");
    expect(notices).not.toContain("not a licence");
  });

  it("orders entries by name and then by version", () => {
    const notices = renderNotices({
      crates: [crate("foo", "1.10.0", "ten"), crate("bar", "2.0.0", "bar"), crate("foo", "1.9.0", "nine")],
      js: [crate("zeta", "1.0.0", "zeta"), crate("alpha", "1.0.0", "alpha")],
    });

    expect(tableRows(notices, "Rust crates").map((row) => row.split(" | ").slice(0, 2).join(" "))).toEqual([
      "| bar 2.0.0",
      "| foo 1.9.0",
      "| foo 1.10.0",
    ]);
    expect(tableRows(notices, "JavaScript packages").map((row) => row.split(" | ")[0])).toEqual([
      "| alpha",
      "| zeta",
    ]);
    expect(notices.indexOf("Used by: foo 1.9.0")).toBeLessThan(notices.indexOf("Used by: foo 1.10.0"));
    expect(notices.indexOf("Used by: bar 2.0.0")).toBeLessThan(notices.indexOf("Used by: foo 1.9.0"));
    expect(notices.indexOf("Used by: foo 1.10.0")).toBeLessThan(notices.indexOf("Used by: alpha 1.0.0"));
  });

  it("walks the whole pnpm dependency tree without duplicates", () => {
    const packages = jsPackagesFrom(fixture("pnpm-list.json"));

    expect(packages.map((p) => `${p.name} ${p.version}`)).toEqual([
      "@tauri-apps/api 2.12.5",
      "react 19.3.1",
      "react-dom 19.3.1",
      "scheduler 0.28.0",
    ]);
    expect(packages.find((p) => p.name === "scheduler").path).toBe(
      "node_modules/.pnpm/scheduler@0.28.0/node_modules/scheduler",
    );
  });

  it("the committed notices cover the production dependencies", () => {
    const path = join(repo, "THIRD-PARTY-NOTICES.md");
    expect(existsSync(path)).toBe(true);
    const notices = readFileSync(path, "utf8");
    const { dependencies } = JSON.parse(readFileSync(join(repo, "ui", "package.json"), "utf8"));

    expect(Object.keys(dependencies).length).toBeGreaterThan(0);
    for (const name of Object.keys(dependencies)) expect(notices).toContain(`| ${name} |`);
    for (const name of ["tauri", "wry", "windows"]) expect(notices).toContain(`| ${name} |`);
    expect(tableRows(notices, "Rust crates").length).toBeGreaterThanOrEqual(200);
  });
});
