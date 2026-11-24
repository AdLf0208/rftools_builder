#!/usr/bin/env bash
# Diagnostic 2: does `ctl tp` set Y, and does teleporting load a distant chunk for fill?
set -uo pipefail
CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
CTL=${CTL:?}; GAME=${GAME:?}; RES=${RES:?}; SOCK=${SOCK:?}
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-probe2.XXXXXX)
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"; rm -f "$SOCK"; cd "$SCRATCH"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() { ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null; echo "EXIT:$?"; }
trap cleanup EXIT
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe2 --seed 20261105 \
  --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
ctl wait-ready 180 >/dev/null
echo "spawn state: $(ctl state | jq -c '.player.position')"
echo "tp reply: $(ctl tp 20.5 102 11.5 2>&1)"
ctl wait 10 >/dev/null
echo "after tp: $(ctl state | jq -c '.player.position')"
echo "tp2 reply: $(ctl tp 220.5 100 196.5 2>&1)"
ctl wait 20 >/dev/null
echo "after tp2: $(ctl state | jq -c '.player.position')"
echo "distant block before fill: $(ctl block 219 101 195 2>&1)"
echo "fill reply: $(ctl command "fill 219 101 195 221 103 197 minecraft:stone" 2>&1)"
ctl wait 10 >/dev/null
echo "distant block after fill: $(ctl block 219 101 195 2>&1)"
echo "tp back: $(ctl tp 20.5 102 11.5 2>&1)"
ctl wait 10 >/dev/null
echo "after tp back: $(ctl state | jq -c '.player.position')"
echo "now look-at builder site: $(ctl look-at block 20 102 1 south 2>&1)"
