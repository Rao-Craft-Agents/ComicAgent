# craft-comic/1: a stick-figure comic script

The standard a `comic` strip is written in: what the host's model writes, and what every strip's PNG carries inside it,
so a strip is drawn again, or changed, from the picture alone. The JSON Schema is [`comic.schema.json`](comic.schema.json);
a real script is [`examples/hashmap.comic.json`](examples/hashmap.comic.json).

## Where it lives

| | |
| --- | --- |
| A file of its own | `*.comic.json`, with `"schema": "craft-comic/1"` |
| Inside a strip | the PNG's `iTXt` chunk keyed `comic-spec`, uncompressed, before `IEND` (`comic extract <png>` prints it) |

The script says meaning only: who stands in a panel, how, what they say. Where anything goes on the page, how the
words are lettered and how the ink wobbles are the agent's, so a script never names a size or a coordinate.

## The script

| Key | Holds | Limit |
| --- | --- | --- |
| `schema` | `craft-comic/1` (stamped by the agent; a model leaves it out) | |
| `title` | the strip's name, lettered above it | 48 characters |
| `alt` | the aside a reader finds after the strip; never drawn | 240 characters |
| `panels` | one beat each, in reading order | 1 to 6 |

A panel:

| Key | Holds | Limit |
| --- | --- | --- |
| `caption` | the narrator's box: `text`, and `at` (`above`, the default, or `below`) | 140 characters, 2 lines |
| `figures` | `id` (short, lowercase, the same across panels for one person), `pose`, `facing` (`left`, `right`), `at` (`left`, `center`, `right`), `label` (under the feet), `sign` (with `holding-sign`) | 3 a panel; label 20, sign 24 |
| `bubbles` | `by` (a figure's id, or `off` for a voice outside the panel), `text`, `kind` (`speech`, the default, `thought`, `shout`) | 3 a panel; 110 characters, 4 lines |
| `props` | `kind`, `at`, `text` (what a whiteboard, signpost, cloud or arrow says; for a graph `up`, `down`, `spike` or `wiggle`) | 3 a panel; 48 characters |

Poses: `standing`, `pointing`, `arms-up`, `shrugging`, `facepalm`, `walking`, `sitting`, `lying-down`, `holding-sign`,
`waving`. Props: `desk`, `laptop`, `whiteboard`, `box`, `arrow`, `cloud`, `graph`, `door`, `tree`, `signpost`.

## Read loosely, drawn whole

A script is read the way a model writes one: a bad element of a list is dropped, a pose or prop the catalogue lacks is
kept by its word. Then:

- **Measured.** Every limit above, a `by` no figure in the panel has, an unknown pose or prop, and what the layout
  measured (a bubble taller than the sky above the figures, a caption past its band) is a finding: one sentence with
  its fix ("panel 2's bubble by ben runs to 6 lines and a bubble holds 4: say it in about 90 characters"). Findings
  go back to the model to revise, up to the call's `rounds`.
- **Clamped.** Whatever the last round still breaks is cut to fit so a PNG is always written, each cut a `problem` in
  the answer (`short`, `skipped`, `fallback`: an unknown pose is drawn standing).

## Drawing

- **Lettering.** Hershey Simplex Roman, a single-stroke plotter font, uppercase, each glyph turned and nudged a little
  by a seeded hand. The fonts' acknowledgement is in `crates/comic-core/fonts/HERSHEY-LICENSE.txt`.
- **Ink.** Every line is a run of capsules, subdivided and wobbled perpendicular to its direction, seeded per panel,
  figure and stroke, so revising one panel does not reshake the others. Bubbles, captions and clouds are filled with
  paper so ink behind them does not show through.
- **The same picture twice.** The same script and seed draw the same bytes on the CPU. The GPU (wgpu, a compute
  shader) draws the same signed-distance math, within one 8-bit step of the CPU on this Mac's Metal.

## Versioning

`craft-comic/<major>`: a new optional key, pose, prop or kind is a minor change and keeps `craft-comic/1`. A reader
keeps a pose or prop it does not know by its word and draws it as its fallback.
