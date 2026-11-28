# Theming a template

A template can change how the panel looks with an optional `theme` block. The rest of the
template format, field by field, is in [The template format](templates.md). The theme overrides
some of the design tokens in [`design/tokens.json`](../design/tokens.json); every token it does
not mention keeps its default. A template without a `theme` looks like the default design.

The tokens file is the single source of the design: a [DTCG 2025.10](https://www.designtokens.org/tr/2025.10/format/)
file, embedded in the binary. Its `$extensions["dev.plinth"]` entry says whether a template may
change each token (`themeable`) and between which bounds (`min`, `max`).

## Writing a theme

`theme` is a flat map from a token path to a single value:

```yaml
schema_version: "1"
# … metadata, pages and sidebar as usual …
theme:
  color.button.background: "#1e1e2e"   # quoted: an unquoted # starts a YAML comment
  size.button.min: "72px"
  font.weight.label: 700
  font.line-height.label: 1.4
  motion.page-transition: "150ms"
```

The rules:

- **One value per token.** Never a DTCG object (`{ "value": 72, "unit": "px" }`), an alias
  (`{color.button.background}`) or a CSS expression.
- **Quote every colour.** `color.button.background: #1e1e2e` is read by YAML as a key with no
  value, and the template is rejected with a hint to quote it.
- **Each token once.** A key written twice anywhere in the template, `theme` included, rejects
  the template; YAML would otherwise keep the last one silently.
- **At most 32 entries**, with keys like `color.button.background`: lowercase, two to four
  dot-separated parts.

### Values by type

| type | write | example | rejected |
|---|---|---|---|
| `color` | `#rrggbb`, quoted (upper or lower case) | `"#1e1e2e"` | `"#fff"`, `"rgb(30, 30, 46)"`, unquoted |
| `dimension` | whole pixels, with the unit `px` | `"72px"` | `72`, `"4.5rem"`, `"72.5px"`, `"72 px"` |
| `duration` | whole milliseconds, with the unit `ms` | `"150ms"` | `150`, `"0.15s"` |
| `fontWeight` | a whole number from 1 to 1000, or a DTCG name | `700`, `"bold"` | `"700"`, `"Bold"` |
| `number` | a plain number, at most two decimals | `1.4` | `"1.4"`, `1.234` |

The font weight names are the DTCG ones: `thin`/`hairline` 100, `extra-light`/`ultra-light` 200,
`light` 300, `normal`/`regular`/`book` 400, `medium` 500, `semi-bold`/`demi-bold` 600, `bold`
700, `extra-bold`/`ultra-bold` 800, `black`/`heavy` 900, `extra-black`/`ultra-black` 950. A
name still has to fall inside the token's range.

### The four fixed tokens

No template can change these; a theme that mentions one is rejected:

- `color.quit.background` and `color.quit.text`: `Quit` always looks like `Quit`, so a theme
  cannot disguise the way out.
- `size.touch.min` (44px, WCAG 2.5.5 target size): no sidebar button shrinks below a finger.
- `font.family.label`: the panel uses the system font; fonts are not loaded from templates.

### The template has to fit the panel

The panel window is 900×300 px. With the theme applied, every page grid and the sidebar must fit
it; a template that does not fit is **rejected, not clipped**:

- page grid height: `rows·size.button.min + (rows−1)·space.page.gap + 2·space.page.padding ≤ 300`;
- page grid width: `cols·size.button.min + (cols−1)·space.page.gap + 2·space.page.padding ≤ 900 − size.sidebar.width`;
- sidebar height, with `n` buttons: `n·size.touch.min + (n−1)·space.sidebar.gap + 2·space.sidebar.padding ≤ 300`;
- sidebar width: `size.sidebar.width − 2·space.sidebar.padding ≥ size.touch.min`.

`rows` and `cols` are the largest `row + row_span` and `col + col_span` of the page, where its
farthest button ends. With the defaults, a page
holds up to 3 rows (`3·64 + 2·8 + 24 = 232`) and 10 columns (`10·64 + 9·8 + 24 = 736 ≤ 772`), and
the sidebar up to 5 buttons (`5·44 + 4·8 + 16 = 268`; six need 320). This applies even without a
`theme`.

A template that does not fit shows as `(invalid)` in the tray's `Templates` menu and is not
picked at startup. The panel does not print the reason; it is the error the template loader
returns, for example:

```
the theme does not fit the panel: sidebar height needs 320px but only 300px are available
```

### Error messages

A key or value with the wrong shape (spaces, `;`, braces, parentheses, more than 32 entries) is
stopped first by the template schema: ``the template does not satisfy the schema at `/theme/<token>` ``.
Every other problem with the `theme` block starts with `invalid theme: `, then:

| problem | message after the prefix |
|---|---|
| unknown token | `` unknown theme token `color.button.nope` `` |
| fixed token | `` theme token `color.quit.background` is fixed and cannot be changed by a template `` |
| wrong type | `` theme token `size.button.min` expects a dimension in whole pixels, like "64px" `` |
| out of range | `` theme token `size.button.min`: 40px is outside the allowed range 44px to 96px `` |
| unquoted colour | `` `color.button.background` has no value; a colour must be quoted, like "#1e1e2e", because an unquoted # starts a YAML comment `` |

## Per-button style

A button on a page can change its own colours with `style`, a map like `theme` that applies
to that button only:

```yaml
pages:
  - components:
      - id: big_cooldown
        position: { row: 0, col: 0 }
        keys: "1"
        label: "Cooldown"
        style:
          color.button.background: "#7a1f2b"
          color.button.text: "#ffffff"
```

It takes only these four tokens:

- `color.button.background`
- `color.button.text`
- `color.button.border`
- `color.button.pressed-background`

The rules:

- **The same values as `theme`.** Each entry is checked like a theme entry (the token exists,
  it is themeable, the value has its type), and a colour is quoted `#rrggbb`. The messages are
  the theme's, starting with ``invalid style on `<id>`: `` instead of `invalid theme: `.
- **Only those four.** Any other token, even one a `theme` may change, is rejected:
  ``invalid style on `big_cooldown`: theme token `size.button.min` cannot be set on one button; its `style` only takes …``.
  Sizes, radii and type stay the theme's.
- **One to four entries.** An empty `style` is rejected by the schema.
- **Only what it gives.** A button's `style` changes only the tokens it names; the rest keep
  the template's `theme`, and its neighbours keep the whole theme.
- **Not everywhere.** A sidebar button cannot have a `style`, nor can the quit button, so it
  always looks like `Quit`.

Rust writes those tokens as CSS from the parsed values, like the theme, and the UI sets them on
the button itself, through the same `--plinth-…` filter as the theme.

## The tokens

`allowed` is what a template may write: `#rrggbb` for any colour, a range (inclusive) for the
rest, and `fixed` for the four tokens no template can change. A test
(`crates/plinth-infra/src/design_tokens.rs`) checks this table against `design/tokens.json`.

| token | type | allowed |
|---|---|---|
| `color.panel.background` | `color` | `#rrggbb` |
| `color.sidebar.background` | `color` | `#rrggbb` |
| `color.button.background` | `color` | `#rrggbb` |
| `color.button.border` | `color` | `#rrggbb` |
| `color.button.text` | `color` | `#rrggbb` |
| `color.button.pressed-background` | `color` | `#rrggbb` |
| `color.button.toggled-background` | `color` | `#rrggbb` |
| `color.button.toggled-text` | `color` | `#rrggbb` |
| `color.quit.background` | `color` | `fixed` |
| `color.quit.text` | `color` | `fixed` |
| `size.sidebar.width` | `dimension` | `96px to 240px` |
| `size.button.min` | `dimension` | `44px to 96px` |
| `size.button.border-width` | `dimension` | `0px to 4px` |
| `size.touch.min` | `dimension` | `fixed` |
| `radius.button` | `dimension` | `0px to 24px` |
| `space.page.padding` | `dimension` | `0px to 24px` |
| `space.page.gap` | `dimension` | `4px to 16px` |
| `space.sidebar.padding` | `dimension` | `0px to 16px` |
| `space.sidebar.gap` | `dimension` | `4px to 16px` |
| `font.size.label` | `dimension` | `12px to 28px` |
| `font.weight.label` | `fontWeight` | `300 to 900` |
| `font.line-height.label` | `number` | `1 to 1.6` |
| `font.family.label` | `fontFamily` | `fixed` |
| `motion.page-transition` | `duration` | `0ms to 400ms` |
| `motion.press-feedback` | `duration` | `0ms to 200ms` |

The defaults are in `design/tokens.json`. Their contrast is checked by tests: text at least 4.5:1
against its background and the button border at least 3:1 against the panel and the sidebar. A
template's own colours are not checked for contrast; that is up to its author.

## How a theme reaches the panel

1. The loader checks the shape with the template schema, then each entry in order: the token
   exists, it is themeable, the value parses as the token's type, and it is in range.
2. The resolved theme (every token, default or overridden) is written back as CSS from the
   parsed numbers, never from the template's text: `--plinth-color-button-background: #1e1e2e`.
   The variable name is the token path with dots turned into dashes.
3. The UI sets those variables on `.panel`, and only names `--plinth-…` with values made of
   letters, digits, `#`, `.`, `,`, `-` and spaces; nothing with a parenthesis, so no `url(`.
