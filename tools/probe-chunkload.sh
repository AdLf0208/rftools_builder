#!/usr/bin/env bash
# Diagnostic: why did the distant fill and the card click not take?
set -uo pipefail
CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
CTL=${CTL:?}; GAME=${GAME:?}; RES=${RES:?}; SOCK=${SOCK:?}
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-probe.XXXXXX)
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"; rm -f "$SOCK"; cd "$SCRATCH"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
cleanup() { ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null; echo "EXIT:$?"; }
trap cleanup EXIT
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe --seed 20261105 \
  --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
ctl wait-ready 180 >/dev/null
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
echo "PX=$PX PZ=$PZ"
echo "fill reply: $(ctl command "fill $((PX+199)) 101 $((PZ+198)) $((PX+201)) 103 $((PZ+200)) minecraft:stone" 2>&1)"
ctl wait 10 >/dev/null
echo "block after fill: $(ctl block $((PX+199)) 101 $((PZ+198)) 2>&1)"
echo "block second: $(ctl block $((PX+200)) 102 $((PZ+199)) 2>&1)"
# Relaunch
ctl quit >/dev/null 2>&1; sleep 2
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe --seed 20261105 \
  --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
ctl wait-ready 180 >/dev/null
echo "block after relaunch: $(ctl block $((PX+199)) 101 $((PZ+198)) 2>&1)"
# Builder rig
BX=$PX; BY=102; BZ=$((PZ+1))
ctl command "fill $((BX-4)) $((BY-3)) $((BZ-4)) $((BX+4)) $((BY+4)) $((BZ+4)) minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl setblock "$BX" $((BY-1)) "$BZ" minecraft:stone >/dev/null
echo "setblock builder: $(ctl command "setblock $BX $BY $BZ rftoolsbuilder:builder" 2>&1)"
ctl wait 6 >/dev/null
echo "builder block: $(ctl block "$BX" "$BY" "$BZ" 2>&1)"
ctl setblock "$BX" $((BY+1)) "$BZ" minecraft:chest >/dev/null
ctl command "clear @a" >/dev/null
ctl tp "$BX.5" "$BY" "$((BZ+3)).5" >/dev/null; ctl wait 6 >/dev/null
echo "give: $(ctl give rftoolsbuilder:shape_card_def 1 2>&1)"; ctl wait 6 >/dev/null
echo "select: $(ctl select rftoolsbuilder:shape_card_def 2>&1)"; ctl wait 6 >/dev/null
echo "inventory: $(ctl inventory 2>&1 | head -c 600)"
echo "state: $(ctl state 2>&1 | head -c 400)"
echo "chat shape: $(ctl chat "/rftoolsbuilder shape box 3 3 3" 2>&1)"; ctl wait 10 >/dev/null
echo "chat offset: $(ctl chat "/rftoolsbuilder offset 200 0 200" 2>&1)"; ctl wait 10 >/dev/null
echo "chat type: $(ctl chat "/rftoolsbuilder type quarry" 2>&1)"; ctl wait 10 >/dev/null
echo "messages after writes: $(ctl messages 20 2>&1 | head -c 1200)"
echo "look-at: $(ctl look-at block "$BX" "$BY" "$BZ" south 2>&1)"; ctl wait 6 >/dev/null
echo "state before interact: $(ctl state 2>&1 | head -c 400)"
echo "interact: $(ctl interact "$BX" "$BY" "$BZ" south 2>&1)"; ctl wait 12 >/dev/null
echo "inventory after: $(ctl inventory 2>&1 | head -c 600)"
echo "messages after: $(ctl messages 20 2>&1 | head -c 1200)"
echo "actionbar after: $(ctl messages 20 2>&1 | jq -r '.actionbar // \"\"')"
echo "status: $(ctl chat "/rftoolsbuilder status $BX $BY $BZ" 2>&1)"; ctl wait 10 >/dev/null
echo "messages status: $(ctl messages 20 2>&1 | head -c 900)"
