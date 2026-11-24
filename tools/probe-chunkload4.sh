#!/usr/bin/env bash
# Diagnostic 4: unload by walking away (no relaunch). Does `loaded` go false?
set -uo pipefail
CTL=${CTL:?}; GAME=${GAME:?}; RES=${RES:?}; SOCK=${SOCK:?}
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-probe4.XXXXXX)
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"; rm -f "$SOCK"; cd "$SCRATCH"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() { ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null; echo "EXIT:$?"; }
trap cleanup EXIT
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe4 --seed 20261105 \
  --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
ctl wait-ready 180 >/dev/null
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1); PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
FX=$((PX+199)); FY=101; FZ=$((PZ+198))
ctl tp "$((PX+200)).5" 102 "$((PZ+199)).5" >/dev/null; ctl wait 20 >/dev/null
echo "fill: $(ctl command "fill $FX $FY $FZ $((FX+2)) $((FY+2)) $((FZ+2)) minecraft:stone" 2>&1)"
ctl wait 10 >/dev/null
echo "loaded at fill: $(ctl block "$FX" "$FY" "$FZ" | jq -c '{block,loaded}')"
ctl tp "$PX.5" 101 "$((PZ+4)).5" >/dev/null; ctl wait 10 >/dev/null
echo "player back: $(ctl state | jq -c '.player.position')"
for n in 0 200 400 400 400; do
  [ "$n" -gt 0 ] && ctl wait "$n" >/dev/null
  echo "distant after +$n: $(ctl block "$FX" "$FY" "$FZ" | jq -c '{block,loaded}')"
done
