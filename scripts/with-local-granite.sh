#!/usr/bin/env bash
# Runs a command with GraniteRust's GitHub URL read from a clone on this machine, for work on both before a push:
#
#   GRANITE_RUST=~/Documents/repositories/GraniteRust scripts/with-local-granite.sh cargo test
#
# Git's own URL rewriting, set for this command only (GIT_CONFIG_*); cargo must fetch with the git CLI
# (CARGO_NET_GIT_FETCH_WITH_CLI). Cargo.lock records the GitHub URL at the clone's commits: push them and the
# lock reads from GitHub as it is.
set -euo pipefail
: "${GRANITE_RUST:?set GRANITE_RUST to a GraniteRust clone}"
clone=$(cd "$GRANITE_RUST" && pwd)
export CARGO_NET_GIT_FETCH_WITH_CLI=true
export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0="url.file://$clone.insteadOf"
export GIT_CONFIG_VALUE_0="https://github.com/riteshpakala/GraniteRust.git"
exec "$@"
