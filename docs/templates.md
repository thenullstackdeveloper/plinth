# The template format

A template describes a panel: its pages of buttons, its sidebar, and how it looks. This is the
**template format 1.0**, field by field. The [template schema](../schema/plinth.template.schema.json)
is the source of truth for the shape; this page explains it, gives the rules the schema cannot
express, and says what will and will not change in later versions.

Format 1.0 is frozen: a template written for it keeps loading in every later Plinth (see
[Compatibility](#compatibility)).

## A template is a file or a folder

Templates live in **`%APPDATA%\plinth\templates`** on Windows. Each one is either:

- **a single YAML file**, `x.yaml`. It is listed as `x.yaml` and cannot reference images;
- **a folder with a `template.yaml` file inside**, `x/template.yaml`. It is listed as `x/`, with
  the slash, and is the only kind that can reference images, which sit in the same folder.

So a file `wow.yaml` and a folder `wow` are two different templates, `wow.yaml` and `wow/`.

A folder counts as a template only if:

- its name starts with a letter or a digit, holds only letters, digits, `.`, `_`, `-` and
  spaces, is at most 64 characters long, and does not end in `.yaml`;
- it is a real folder, not a symbolic link or a junction;
- its `template.yaml` is a regular file, not a link.

```text
%APPDATA%\plinth\templates\
  wow-default-keys.yaml        listed as wow-default-keys.yaml
  my-mage\                     listed as my-mage/
    template.yaml
    images\
      fireball.png
      frost.webp
```

## The format version

Every template starts with `schema_version`, **quoted**:

```yaml
schema_version: "1.0"
```

- It is `"1.<minor>"`, and `"1"` is the same as `"1.0"`, for good. The minor has no leading
  zeros and at most four digits: `"1.01"` and `"1.12345"` are refused.
- This Plinth reads up to **1.0**. A template of a later minor is refused before its fields are
  looked at, so it asks for a newer Plinth instead of failing on a field this one does not know.
- It has to be quoted: unquoted, YAML reads `1.0` as a number (and `1.10` as `1.1`).

| `schema_version` | result |
|---|---|
| `"1"`, `"1.0"` | loads, as format 1.0 |
| `"1.1"` | `this template needs a newer Plinth (format 1.1; this one reads up to 1.0)` |
| `"2"` | `format 2 is not supported` |
| `"1.01"` | `format 1.01 is not supported` |
| `1.0` (unquoted) | `` `schema_version` must be quoted, like "1.0" `` |

## The top level

| field | required | what it is |
|---|---|---|
| `schema_version` | yes | the format version, quoted: see above |
| `metadata` | yes | `game`, `version` and `author`, all text and all required |
| `pages` | yes | the pages, at least one; the panel opens on the first |
| `sidebar` | yes | the column of buttons beside the pages |
| `background` | no | the default background of every page: see [Backgrounds](#backgrounds) |
| `effects` | no | the change of page and the press feedback: see [Effects](#effects) |
| `theme` | no | colours, sizes and timings of the whole panel: see [Theming a template](theming.md) |

Any other field is refused, and so is any key written twice anywhere in the template: YAML would
otherwise keep the last one without a word.

## Pages and buttons

`pages` is a list; each page has `components`, its buttons (at least one), and may have its own
`background`. A button:

```yaml
- id: fireball
  position: { row: 0, col: 0, row_span: 2, col_span: 2 }
  keys: "1"
  label: "Fireball"
  image: "images/fireball.png"
```

| field | required | what it is |
|---|---|---|
| `id` | yes | a name for the button, unique in the whole template |
| `position` | yes | its cell in the page's grid, and how many cells it covers |
| `keys` or `action` | one of the two | what it does; never both |
| `label` | no | its text. Without one, the button shows and is named by its `id` |
| `icon` | no | a small image next to the label, or alone with `hide_label` |
| `image` | no | an image that fills the face of the button. Not with `icon` |
| `hide_label` | no | `true` hides the label from view; it stays the button's name |
| `style` | no | the button's own colours over the theme |

### `position`

- `row` and `col` are the button's first cell, counted from 0 at the top left.
- `row_span` (1 to 4) and `col_span` (1 to 10) are how many rows and columns it covers from
  there; both are 1 when absent. Spans are only for page buttons.
- Two buttons of a page cannot cover the same cell:
  `` components `a` and `b` share a cell on page 0 `` (pages count from 0 too).
- A page's grid is as large as its farthest button: the largest `row + row_span` rows and
  `col + col_span` columns. With the default sizes a page holds 3 rows and 10 columns; a
  template that does not fit the 900×300 panel is **rejected, not cut** (the exact sums are in
  [Theming a template](theming.md#the-template-has-to-fit-the-panel)).

### `keys` and `action`

- **`keys`** is what the button presses, as literal keys: one step (`"m"`), a chord with `shift`,
  `ctrl` or `alt` (`"shift+1"`), or a list of up to 16 steps sent in order
  (`["c", "b"]`). Keys are letters, digits, `f1` to `f12`, `space`, `enter`, `esc`, `tab`,
  `backspace` and the punctuation keys of the main block. A modifier written twice in a chord is
  refused.
- **`action`** is one of the panel's own actions or a named game action:
  - `app.next_page`, `app.prev_page` and `app.go_to_page:<n>` (`app.go_to_page:0` is the first
    page; a page the template does not have is refused);
  - `app.toggle_lock` (the lock: while it is on, the page cannot change);
  - `app.switch_sidebar` (moves the sidebar to the other side);
  - `app.quit` (closes Plinth, even while the panel is locked);
  - a World of Warcraft action: `wow.world_map`, `wow.character`, `wow.bags`, `wow.spellbook`,
    `wow.quest_log` or `wow.talents`, sent with the game's default keys.

  Any other action is refused: `` unknown action: `wow.nope` (not in the catalog) ``.

### `label`, `icon`, `image` and `hide_label`

A button shows a label, an icon, an image, or a combination:

| you write | the button shows |
|---|---|
| `label` | the text |
| `label` + `icon` | the icon to the left of the text, centred together |
| `label` + `icon` + `hide_label: true` | the icon alone, larger, in the middle |
| `label` + `image` | the image over the whole face, and the text on a dark strip at the bottom (`color.button.background` at 75 %) so it reads on any picture |
| `label` + `image` + `hide_label: true` | the image alone |

- **`icon` and `image` do not go together**: a button shows one or the other.
- **`hide_label` hides, it does not remove.** The label stays the button's accessible name, the
  one a screen reader reads, even when it changes (`Lock` becomes `Unlock`). Without a `label`,
  the name is the `id`.
- The image is cropped to the face inside the border, never stretched; the icon is scaled to fit
  its box, never cropped.
- Both are paths to images in the template's folder: see [Images](#images).

### `style`

A page button can change its own four colours over the template's `theme`:

```yaml
style:
  color.button.background: "#7a1f2b"
  color.button.text: "#ffffff"
  color.button.border: "#d08770"
  color.button.pressed-background: "#9a2f3b"
```

Only those four tokens, from one to four of them, each checked like in `theme`. Every other
token keeps the theme. The rules and messages are in
[Theming a template](theming.md#per-button-style).

## The sidebar

```yaml
sidebar:
  default_side: right
  components:
    - { id: lock, position: { row: 0, col: 0 }, action: "app.toggle_lock", label: "Lock" }
    - { id: switch_side, position: { row: 1, col: 0 }, action: "app.switch_sidebar", label: "Switch side" }
    - { id: quit, position: { row: 2, col: 0 }, action: "app.quit", label: "Quit" }
```

- `default_side` is `left` or `right`, where the sidebar starts.
- `components` are buttons like a page's, in a single column (it can be empty). With the default
  sizes it holds up to 5. They stay on screen whatever the page.
- A sidebar button may have `label`, `icon` and `hide_label`, but **not `image`, `style`,
  `row_span` or `col_span`**, not even a span of 1:
  `` `lock` is in the sidebar, which does not allow `image` ``.

### The quit button

A button with `action: app.quit` always looks like `Quit`, so no template can disguise the way
out. It takes **no `icon`, `image`, `style` or `hide_label`**, on a page or in the sidebar:
``the quit button cannot have `icon`, so it always looks the same``. Its colours are the fixed
`color.quit.*` tokens, at rest and pressed, with every press effect.

### Where each field can be used

| field | page button | sidebar button | quit button |
|---|---|---|---|
| `label` | yes | yes | yes |
| `icon` | yes | yes | no |
| `hide_label` | yes | yes | no |
| `image` | yes | no | no |
| `style` | yes | no | no |
| `row_span`, `col_span` | yes | no | on a page, yes |

## Backgrounds

```yaml
background:
  image: "images/stone.png"      # the default of every page
pages:
  - components: [ … ]            # shows stone.png
  - background:
      image: "images/glow.webp"  # this page's own, instead of the default
    components: [ … ]
```

- `background` at the top level is the default background of every page; `background` on a page
  replaces it for that page. A page with neither has none.
- It covers the page behind its buttons (cropped, never stretched) and changes with the page,
  with its `page_transition`. The sidebar keeps its plain colour.

## Effects

```yaml
effects:
  page_transition: fade
  press: sink
```

From a closed catalogue, for every page and every button, the sidebar's too:

| field | value | what it does |
|---|---|---|
| `page_transition` | `slide` (default) | the pages slide across |
| | `fade` | the pages fade into each other without moving |
| | `none` | the new page replaces the old one at once |
| `press` | `lighten` (default) | a held button takes `color.button.pressed-background` |
| | `sink` | a held button shrinks a little and keeps its colour |
| | `none` | no visual reaction |

Both are optional, and a missing one is its default. Their durations are the theme's
`motion.page-transition` and `motion.press-feedback`. `Quit` and a toggle that is on (the lock)
keep their colours with every press effect.

## Images

Only a template **in a folder** can reference images: in a single-file template any `image`,
`icon` or `background` is refused with
`a single-file template cannot reference images; put it in a folder with template.yaml`.

### Formats

**PNG and WebP, static only.** Plinth recognises an image by its bytes, and the extension has to
match them. Refused:

- an animated image: an APNG, or an animated WebP;
- SVG (it can carry scripts), GIF, JPEG and anything else;
- a file whose bytes are not what its extension says: `` `x.webp` is not a webp file ``;
- a malformed header, or a width or height of 0: `` `x.png` is not supported: zero size ``.

### Paths

An image path is relative to the template's folder and uses `/`, like `images/fire.png`:

- 1 to 4 parts, each starting with a letter or a digit and holding only letters, digits, `.`,
  `_` and `-`, up to 64 characters;
- no `..`, no drive, no leading `/` or `\`;
- it ends in `.png` or `.webp`, in lower case;
- no part is a name Windows reserves for a device (`con`, `prn`, `aux`, `nul`, `com1` to `com9`,
  `lpt1` to `lpt9`), in any case and with any extension.

Before reading it, Plinth checks that no part of the path below the template's folder is a
symbolic link or a junction, that it stays inside the folder, and that it is a regular file.

### Limits

| use | largest image | largest file |
|---|---|---|
| `icon` | 256×256 | 256 KiB |
| `image` | 1024×1024 | 1 MiB |
| `background` | 2048×2048 | 4 MiB |

- **Per template:** at most **64 distinct image files and 16 MiB** between them; `template.yaml`
  does not count.
- A file used in several places counts once, but has to meet the limits of each use: an image
  that is fine as a background can be too large for an icon.
- The width and height are read from the image's header.

### Messages

| problem | message |
|---|---|
| a path that breaks the rules | `` `images/../x.png` is not a valid image path: `..` is not allowed `` |
| a link, or a path out of the folder | `` `images/x.png` leaves the template folder `` |
| a folder, not a file | `` `images/x.png` is not a regular file `` |
| too large for its use | `` `images/x.png` is too large for an icon: 512x512 > 256x256 `` |
| bytes and extension differ | `` `images/x.png` is not a png file `` |
| animated or malformed | `` `images/x.png` is not supported: animated `` |
| too many or too heavy | `the template's images exceed 64 files or 16 MiB` |
| `icon` and `image` together | ``the template does not satisfy the schema at `/pages/0/components/0` `` … |

## When a template is refused

A template that breaks any rule shows as `(invalid)` in the tray's `Templates` menu, and the panel
keeps working with another one. The checks run in this order, and the first problem is the one
reported:

1. the YAML itself, and no key written twice;
2. the format version;
3. an unquoted colour in `theme`, which gets a hint to quote it;
4. the shape, against the schema: ``the template does not satisfy the schema at `<where>` `` …;
5. the actions, the keys, the theme, the images and the rules above;
6. whether it fits the panel.

## Compatibility

The rule that keeps a template working:

- **Within format 1 only something optional is added**, and each addition raises the minor:
  1.1, 1.2, … A template of 1.0 never needs to change, and loads in every Plinth that reads
  1.0 or later.
- **Anything that would break an existing template is format 2.** Plinth will keep reading
  format 1 next to it.
- A template that needs a newer Plinth says so, instead of failing on a field it does not know.

### A change made before freezing: `icon`

Before format 1.0 was frozen, `icon` was free text that Plinth ignored. In 1.0 it is the path of
an image, so a template with `icon: "fire"` (no `.png` or `.webp`) no longer loads:
`` `fire` is not a valid image path: it must end in `.png` or `.webp`, in lower case ``. Remove the
line, or point it to an image in the template's folder.

## The reference templates

The reference templates of format 1 are in
[`crates/plinth-tauri/tests/templates-v1/`](../crates/plinth-tauri/tests/templates-v1/), and a test
(`crates/plinth-tauri/tests/templates_v1.rs`) loads every one of them and checks that it fits the
panel:

- `wow-default-keys.yaml` and `wow-demo.yaml`, the two templates Plinth installs, as they were
  when 1.0 was frozen;
- `minimal.yaml`, the smallest template that loads;
- `full/`, a folder template that uses every field of 1.0: a button image, a WebP icon,
  `hide_label`, a default and a page background, a `style`, spans and non-default `effects`.
  It is also the most complete example of the format.

**They are never edited and never removed**; that is what makes them a promise. To add one, put
a new `.yaml` file or a new folder with a `template.yaml` in `templates-v1/`: the test picks it
up by itself. When a minor adds a field, a new reference template that uses it goes next to the
old ones, and the old ones have to keep loading as they are.
