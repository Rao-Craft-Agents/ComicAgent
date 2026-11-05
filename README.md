# ComicAgent

![How a hash map finds a key](Docs/Testing/how-a-hash-map-finds-a-key.png)

Explain a concept with a stick-figure comic strip. Ask with `$comic`, and Craft's model writes the strip's script:
a few panels, stick figures from a catalogue of poses, what each says, a caption a panel, a title and an aside. `comic`
lays it out, letters every word by hand, has the model fix whatever ran over, and draws it as a PNG. The picture carries
its script, so a strip can be drawn again, or changed, from the picture alone.

## What you can ask

```
explain how a hash map finds a key, as a comic $comic #canvas
$comic a one-panel gag about off-by-one errors
$comic explain recursion in four panels, for someone who has only written loops
$comic draw comics/how-a-hash-map-finds-a-key.png again, with a panel on what happens when the buckets fill
```

In the Craft app, `#canvas` opens the pane that shows the pictures a session writes, so the strip appears beside the
chat.

## What you get

- **A strip that explains.** One to six panels (three to five unless you say, or one for a gag), with a title, a
  caption a panel and an aside.
- **Lettered by hand.** Every word in capitals, jittered a little glyph by glyph the way a person letters a comic; a
  seed fixes the hand's wobble, so the same script and seed draw the same picture.
- **A PNG at 2x.** 3200 pixels wide for a strip, 1600 for one panel, with alt text to show under it.
- **A picture you can draw again.** The PNG carries its script, so a later call can redraw it, or change it as a brief
  asks, from the picture alone.
- **The GPU when there is one.** The same picture on the CPU when there is none, and the answer says so.

## How it works

- Craft's model writes the script, which says meaning only, never a coordinate. The agent lays it out and measures
  what fits.
- What ran over goes back to the model to fix, for up to three rounds unless the call says otherwise; the best round is
  drawn once. The answer names anything it could only do in part, such as text cut to fit or an unknown pose drawn
  standing.
- A request it cannot carry out is refused with a sentence that says the fix: no model and no script to draw, `panels`
  outside 1 to 6, or the GPU asked for where there is none.
- Each call runs in a process of its own, released when it has answered.

## Works well with

`comic` is the first Rust agent on [GraniteRust](https://github.com/riteshpakala/GraniteRust), the Rust port of
Granite's agent kernel and its standard. A host written in Swift calls it exactly as it calls a Swift agent, so Craft
offers it from a shelf beside the other Rao Craft Agents with no change.

## Get it

Installing a Cargo repository from Craft itself, and listing it in the Agent Store, come with Craft's Cargo install
path. Until then, install it with GraniteRust's installer onto a shelf, and run Craft with `CRAFT_AGENTS_DIR` naming
that shelf, which Craft then reads in place of its own (`~/.rao/apps/craft/agents`):

```bash
granite agents add https://github.com/Rao-Craft-Agents/ComicAgent.git --shelf <shelf>
CRAFT_AGENTS_DIR=<shelf> craft agents      # lists comic
```

Then say `$comic` in a request.

## Requirements

- macOS 14 or later, or Linux on arm64. With no GPU, as in a Linux VM, the strip is drawn on the CPU.
- Rust and GraniteRust's installer, which clones the repository and builds the runner from source, in release.
- A host with a model to draft the script; with no model, give it a `script` to draw.
- The source is MIT. The runner links GraniteRust, which is GPL-3.0, so a built `comic` runner is a GPL-3.0 combined
  work.
- The lettering is the Hershey Simplex Roman font: the Hershey Fonts were originally created by Dr. A. V. Hershey while
  working at the U. S. National Bureau of Standards, and the format of the font data was originally created by James
  Hurt, Cognition, Inc. (the full acknowledgement is in `crates/comic-core/fonts/HERSHEY-LICENSE.txt`).

## For developers

What a call takes and answers, the script, how a host runs it, the `comic` command and how to build and test it are in
the [reference](Docs/Reference.md). The agent standard it follows is [Schema/Comic.md](Schema/Comic.md), its copy of
Craft's agent standard; the script's own standard, `craft-comic/1`, is [Schema/ComicScript.md](Schema/ComicScript.md);
and how it is tested is [TESTING.md](TESTING.md).
