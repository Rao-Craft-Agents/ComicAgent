# craft-agent/1 · comic

Draws a stick-figure comic strip that explains a concept, as a PNG: the host's model writes the script (panels, figures, what they say, the captions), the agent inks and letters it by hand in the style of webcomics, and the picture carries its script so it can be drawn again.

comic's copy of Craft's agent standard (`craft-agent/1`, Craft's `Schema/Agent.md`), written by
`craft agents schema comic` from its record. Everything above **Changes** is written again each time; Changes is
comic's own log.

| File | What it holds |
| --- | --- |
| [`comic.input.schema.json`](comic.input.schema.json) | What it takes, as JSON Schema |
| [`comic.answer.schema.json`](comic.answer.schema.json) | What it answers: `granite-answer/1` with its own kinds |

## What it takes

| Field | Required | What it is |
| --- | --- | --- |
| `brief` | no | what to explain and the angle to take: the concept, who is confused by it, where the joke is; the ask when left out |
| `panels` | no | how many panels, 1 to 6; left out, the model picks three to five, or one for a gag |
| `script` | no | a strip's PNG from an earlier call (it carries its script), or a .comic.json, to draw again; with a brief, changed as the brief asks |
| `output` | no | where to write the PNG; comics/<title>.png when left out |
| `rounds` | no | how many times the model may fix what the strip measured; 3 when left out |
| `seed` | no | a number that fixes the hand's wobble; the same script and seed draw the same picture |
| `renderer` | no | gpu, cpu, or auto (left out): the GPU when there is one, the same picture on the CPU when there is none |

`directory` is filled by Craft with the project folder. The person's request and the host's model are offered
beside the call, never in it.

## What it makes

| Kind | What | Into | Never |
| --- | --- | --- | --- |
| `comic` | a stick-figure comic strip that explains a concept, as a PNG with its script inside | `comics/*` | a picture of a real person, a chart of numbers, or the script inside the answer |

## What it answers

A `granite-answer/1` answer (Granite's `Schema/GraniteAgent.md` §4.2): `paths`, `made` against `expected` by kind and
name, `problems` for what it could only do in part, `payloads` for files that keep a standard of their own, and its
own keys beside them. Craft measures each call by what it made of what it expected.

## How it refuses

With an `AgentRefusal`: a kind and reasons, each with its fix. `input` and `call` are the call's to correct; `build`,
`device`, `reach`, `model` and `host` what stood in the way; `answer` and `parse` its own bug.

## Changes

Each change made to make this agent more precise, dated, with the regime run that measured it and what it
made of what it was asked before and after (`TESTING.md`). Newest first.

### 2026-11-05: the first version

`comic` as built: the host's model drafts a `craft-comic/1` script ([ComicScript.md](ComicScript.md)), the agent
measures it, sends what ran over back for up to `rounds` revisions, and draws the best round once. No regime run has
measured it yet: precision is 1 on every fixture call (`made` = `expected`, kind `comic`, named for the title), and the
first live turns will say how often a local model's draft needs a second round.

