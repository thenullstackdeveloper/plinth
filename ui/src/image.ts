/**
 * The only image sources the panel paints: a PNG or a WebP inlined as
 * base64. Nothing that could fetch (`https:`, a path), run (`javascript:`) or
 * carry a script (`data:image/svg+xml`), and no space, quote or parenthesis
 * that could break out of a `url("…")`.
 */
const SAFE_IMAGE = /^data:image\/(png|webp);base64,[A-Za-z0-9+/]+={0,2}$/;

/**
 * `uri` if it is an image the panel may paint, otherwise `null`. Rust only
 * sends such URIs; this is the last barrier before the page.
 */
export function safeImageSrc(uri: string | null | undefined): string | null {
  return typeof uri === "string" && SAFE_IMAGE.test(uri) ? uri : null;
}
