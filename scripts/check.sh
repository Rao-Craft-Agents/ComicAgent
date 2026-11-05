#!/usr/bin/env bash
# ComicAgent's check: its own suite, then GraniteRust's check on this repository (installed by GraniteRust's installer
# and called over the wire as a host calls it, every answer held to granite-agent/1 and Craft's craft-agent/1), then
# what only a comic answers.
#
#   GRANITE_RUST=<GraniteRust clone> scripts/check.sh [--native]
#
# In Apple's container (Linux, no Metal) the strip is drawn on the CPU and the answer says so; with --native it is
# drawn on this Mac's GPU, and the GPU's strip is compared with the CPU's.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
: "${GRANITE_RUST:?set GRANITE_RUST to a GraniteRust clone: its containers/check.sh is the check}"
granite=$(cd "$GRANITE_RUST" && pwd)
mode=container; [ "${1:-}" = --native ] && mode=native
export PATH="$HOME/.cargo/bin:$PATH"
cargo_() { "$root/scripts/with-local-granite.sh" cargo "$@"; }

echo "== the suite"
(cd "$root" && cargo fmt --all --check && cargo_ clippy --workspace --all-targets -q -- -D warnings && cargo_ test --workspace -q 2>&1 | grep -E 'test result|FAILED|panicked')
(cd "$root" && cargo_ build --release -q -p comic-agent)
comic="$root/target/release/comic"

out=$(mktemp -d "${TMPDIR:-/tmp}/comic-check.XXXXXX")
flags=(--granite "$granite" --out "$out")
[ $mode = native ] && flags+=(--native)
"$granite/containers/check.sh" "$root" "${flags[@]}"

echo
echo "== what only a comic answers"
answer="$out/redraw.perform.answer.json"
jq -e '.width == 3200 and .panels == 3 and .rounds == 1 and .findings == [] and .made[0].name == "how-a-hash-map-finds-a-key"' "$answer" >/dev/null \
  || { echo "FAILED: the redraw's answer: $(cat "$answer")" >&2; exit 1; }
if [ $mode = container ]; then
  jq -e '.renderer == "cpu" and ([.problems[].kind] | index("fallback"))' "$answer" >/dev/null \
    || { echo "FAILED: in a container the strip is drawn on the CPU, and the answer says so" >&2; exit 1; }
fi
png="$out/work/comics/how-a-hash-map-finds-a-key.png"
read -r width height < <(od -An -tu1 -j16 -N8 "$png" | awk '{print $1*16777216+$2*65536+$3*256+$4, $5*16777216+$6*65536+$7*256+$8}')
[ "$width" = "$(jq .width "$answer")" ] && [ "$height" = "$(jq .height "$answer")" ] || { echo "FAILED: the PNG is ${width}x${height}" >&2; exit 1; }
schemas=(--schemas "$root/Schema" --schemas "$granite/Schema" --schemas "$granite/containers/schemas")
"$granite/target/release/granite" check "${schemas[@]}" --schema comic.answer.schema.json "$answer"
"$comic" extract "$png" > "$out/carried.comic.json"
"$granite/target/release/granite" check "${schemas[@]}" --schema comic.schema.json "$out/carried.comic.json"
echo "the strip is ${width}x${height}, drawn on the $(jq -r .renderer "$answer"), and carries its craft-comic/1 script"
if [ $mode = native ]; then
  (cd "$out" && "$comic" draw "$root/Schema/examples/hashmap.comic.json" --out gpu.png --seed 7 --renderer gpu >/dev/null 2>&1 \
    && "$comic" draw "$root/Schema/examples/hashmap.comic.json" --out cpu.png --seed 7 --renderer cpu >/dev/null 2>&1)
  echo "GPU against CPU: $("$comic" compare "$out/gpu.png" "$out/cpu.png" --tolerance 3)"
fi
echo
echo "comic: check passed ($mode); kept in $out"
