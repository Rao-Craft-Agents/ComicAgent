# ComicAgent reference

What the `comic` agent takes and answers, the script it draws from, how a host runs it, its command, how to build and
test it, and its licences. What it is for is in the [README](../README.md); the agent standard it follows is in
[Schema/Comic.md](../Schema/Comic.md).

The first Rust agent on [GraniteRust](https://github.com/riteshpakala/GraniteRust), the Rust port of Granite's agent
kernel and its standard (`granite-agent/1`). A host written in Swift calls it exactly as it calls a Swift agent.

## What it takes

| Field | What it is |
| --- | --- |
| `brief` | what to explain and the angle to take: the concept, who is confused by it, where the joke is; the person's ask when left out |
| `panels` | how many panels, 1 to 6; left out, the model picks three to five, or one for a gag |
| `script` | a strip's PNG from an earlier call (it carries its script), or a `.comic.json`, to draw again; with a `brief`, changed as the brief asks |
| `output` | where to write the PNG; `comics/<title>.png` when left out |
| `rounds` | how many times the model may fix what the strip measured; 3 when left out |
| `seed` | a number that fixes the hand's wobble; the same script and seed draw the same picture |
| `renderer` | `gpu`, `cpu`, or `auto` (left out): the GPU when there is one, the same picture on the CPU when there is none |

Every field may be left out. With no `script`, the host's model drafts one; a host with no model is refused with the
fix ("give `script`").

## What it makes and answers

It makes `comic` into `comics/*`: a PNG at 2x (3200 px wide for a strip, 1600 for one panel). It answers
`granite-answer/1`: `path`/`paths`, `made` against `expected` (one `comic`, named for the title), `problems` for what it
could only do in part (`short` text cut to fit, `fallback` an unknown pose drawn standing or the CPU drawing for a
missing GPU), and beside them `width`, `height`, `panels`, `title`, `alt` (show it under the picture), `rounds`,
`findings` (what the strip written still measures wrong) and `renderer`. When findings are left, `note` tells the
calling model the call is done, so it does not call again to fix what the drawing kept.

It refuses with a kind and a sentence that says the fix: `call` (no model and no script, a script that does not read,
an output it cannot write), `input` (`panels` outside 1 to 6), `model` (the model's drafts were not strips twice),
`device` (`renderer: gpu` where there is no GPU). [`Schema/Comic.md`](../Schema/Comic.md) is its copy of Craft's agent
standard, written by `craft agents schema comic`.

## The script

[`Schema/ComicScript.md`](../Schema/ComicScript.md) is the standard, `craft-comic/1`, with its JSON Schema
([`Schema/comic.schema.json`](../Schema/comic.schema.json)) and a real script
([`Schema/examples/hashmap.comic.json`](../Schema/examples/hashmap.comic.json)). It says meaning only, never a coordinate.

## How a host runs it

The repository is a Cargo workspace whose `crates/comic` names its runner (`[package.metadata.granite] agent =
"comic"`). An installer clones it, builds the runner in release, asks it what it carries (`comic --describe`), and keeps
it on a shelf with its record; each call is then a process of its own, released when it has answered.

```bash
granite agents add https://github.com/Rao-Craft-Agents/ComicAgent.git --shelf <shelf>   # GraniteRust's installer
echo '{"perform":{"word":"comic","input":{"script":"strip.png"},"model":false}}' | <shelf>/ComicAgent/agent
```

Craft offers a Rust runner from its shelf with no change (`craft agents` lists it, Craft's model drafts the script).
Installing a Cargo repository from Craft itself, and listing it in the store, come with Craft's Cargo install path.

## The command

```text
comic draw <script.comic.json | strip.png> [--out x.png] [--seed N] [--renderer gpu|cpu|auto]
comic extract <strip.png>        the script a strip carries
comic sample                     the fixture script
comic describe                   the agent's entry
comic icon [--out path]          Assets/icon.png, drawn by the agent itself
comic compare <a.png> <b.png> [--tolerance 3]
```

`COMIC_RENDERER=gpu|cpu|auto` chooses the renderer when a call does not.

## Building and testing

```bash
cargo test                                    # the script, lettering, layout, both renderers, the agent, a call over the wire
GRANITE_RUST=<clone> scripts/check.sh         # the suite, then GraniteRust's container check on this repository
GRANITE_RUST=<clone> scripts/check.sh --native
```

To work on GraniteRust and this agent together, `GRANITE_RUST=<clone> scripts/with-local-granite.sh cargo …` reads
GraniteRust's GitHub URL from a clone (git's own URL rewriting, for that command only). [`TESTING.md`](../TESTING.md) has
the runs.

## Licences

The source is MIT. The runner links GraniteRust, which is GPL-3.0, so a built `comic` runner is a GPL-3.0 combined
work. The lettering is the Hershey Simplex Roman font: the Hershey Fonts were originally created by Dr. A. V. Hershey
while working at the U. S. National Bureau of Standards, and the format of the font data was originally created by
James Hurt, Cognition, Inc. (the full acknowledgement is in `crates/comic-core/fonts/HERSHEY-LICENSE.txt`).
