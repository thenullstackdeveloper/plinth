// @vitest-environment node
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const read = (path) => readFileSync(join(repo, path), "utf8");

// Fenced code blocks hold shell comments and examples, not headings or links.
const prose = (markdown) => markdown.replace(/^```[^\n]*\n[\s\S]*?^```[^\n]*$/gm, "");

const README_SECTIONS = [
  "What it is",
  "Install (Windows)",
  "Using the panel",
  "Templates",
  "What Plinth never does",
  "Status",
  "Development",
  "Contributing",
  "License",
];

describe("docs", () => {
  it("the readme has the sections a user needs, in order", () => {
    const headings = prose(read("README.md"))
      .split("\n")
      .filter((line) => line.startsWith("## "))
      .map((line) => line.slice(3).trim());
    expect(headings).toEqual(README_SECTIONS);
  });

  it("every relative link in the readme points to a file that exists", () => {
    const targets = [...prose(read("README.md")).matchAll(/\]\(([^)\s]+)\)/g)]
      .map((match) => match[1])
      .filter((target) => !target.startsWith("http") && !target.startsWith("#"));
    expect(targets.length).toBeGreaterThanOrEqual(6);
    const missing = targets.filter((target) => {
      const path = join(repo, decodeURIComponent(target.split("#")[0]));
      return !existsSync(path) || !statSync(path).isFile();
    });
    expect(missing).toEqual([]);
  });

  it("the readme links the template format guide", () => {
    const readme = prose(read("README.md"));
    const templates = readme.split(/^## /m).find((section) => section.startsWith("Templates\n"));
    expect(templates).toBeDefined();
    expect(templates).toContain("](docs/templates.md)");
    expect(statSync(join(repo, "docs/templates.md")).isFile()).toBe(true);
  });

  it("the readme states the licence and that contributions are closed", () => {
    const readme = read("README.md");
    expect(readme).toContain("PolyForm Noncommercial 1.0.0");
    expect(readme).toContain("not accepting code contributions");
  });

  it("the architecture doc no longer calls plinth-tauri a later phase", () => {
    const architecture = read("docs/architecture.md");
    expect(architecture).not.toContain("does not exist yet");
    expect(architecture).not.toContain("later phase");
  });
});
