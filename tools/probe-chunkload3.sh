#!/usr/bin/env bash
# Diagnostic 3: is the distant chunk loaded after fill + relaunch? Does it unload?
set -uo pipefail
CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
CTL=${CTL:?}; GAME=${GAME:?}; RES=${RES:?}; SOCK=${SOCK:?}
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-probe3.XXXXXX)
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"; rm -f "$SOCK"; cd "$SCRATCH"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
launch() {
  "$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe3 --seed 20261105 \
    --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
  ctl wait-ready 180 >/dev/null
}
cleanup() { ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null; echo "EXIT:$?"; }
trap cleanup EXIT
launch
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1); PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
FX=$((PX+199)); FY=101; FZ=$((PZ+198))
echo "spawn player: $(ctl state | jq -c '.player.position')"
echo "distant before: $(ctl block "$FX" "$FY" "$FZ")"
ctl tp "$((PX+200)).5" 102 "$((PZ+199)).5" >/dev/null; ctl wait 20 >/dev/null
echo "fill: $(ctl command "fill $FX $FY $FZ $((FX+2)) $((FY+2)) $((FZ+2)) minecraft:stone" 2>&1)"
ctl wait 10 >/dev/null
echo "distant after fill: $(ctl block "$FX" "$FY" "$FZ")"
echo "tp back: $(ctl tp "$PX.5" 101 "$((PZ+4)).5" 2>&1)"; ctl wait 10 >/dev/null
echo "player before quit: $(ctl state | jq -c '.player.position')"
ctl quit >/dev/null 2>&1; sleep 2
launch
echo "player after relaunch: $(ctl state | jq -c '.player.position')"
echo "distant after relaunch: $(ctl block "$FX" "$FY" "$FZ")"
ctl wait 400 >/dev/null
echo "distant after wait400: $(ctl block "$FX" "$FY" "$FZ")"
ctl wait 400 >/dev/null
echo "distant after wait800: $(ctl block "$FX" "$FY" "$FZ")"
echo "forceload query: $(ctl command "forceload query" 2>&1)"
