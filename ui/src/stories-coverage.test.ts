import { expect, test } from "vitest";

const components = Object.keys(import.meta.glob("./components/*.tsx")).filter(
  (path) => !path.endsWith(".stories.tsx") && !path.endsWith(".test.tsx"),
);
const stories = new Set(Object.keys(import.meta.glob("./components/*.stories.tsx")));

test("every component has a story file", () => {
  expect(components.length).toBeGreaterThan(0);
  const missing = components.filter((path) => !stories.has(path.replace(/\.tsx$/, ".stories.tsx")));
  expect(missing).toEqual([]);
});
