import { expect, test } from "vitest";

import { safeImageSrc } from "./image";

test("safeImageSrc accepts png and webp data uris", () => {
  for (const uri of ["data:image/png;base64,iVBORw0KGgo=", "data:image/webp;base64,UklGRg==", "data:image/png;base64,ab+/"]) {
    expect(safeImageSrc(uri)).toBe(uri);
  }
});

test("safeImageSrc rejects anything else", () => {
  for (const uri of [
    "https://example.com/x.png",
    "data:image/svg+xml;base64,PHN2Zz4=",
    "javascript:alert(1)",
    "data:image/png;base64,iVBO Rw0K",
    'data:image/png;base64,iVBO"Rw0K',
    "data:image/png;base64,iVBO)Rw0K",
    "data:image/png;base64,iVBORw0KGgo=\n",
    "data:image/png;base64,",
    "data:image/png,iVBORw0KGgo=",
    "data:image/PNG;base64,iVBORw0KGgo=",
    "data:image/png;base64,iVBORw0KGgo===",
    "",
    null,
    undefined,
  ]) {
    expect(safeImageSrc(uri)).toBeNull();
  }
  // Whatever the backend sent that is not text.
  expect(safeImageSrc(42 as unknown as string)).toBeNull();
});
