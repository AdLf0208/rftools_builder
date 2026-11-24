#!/usr/bin/env bash
# Diagnostic 5: a quarry card aimed at a NEVER-VISITED distant chunk (deep stone).
set -uo pipefail
CTL=${CTL:?}; GAME=${GAME:?}; RES=${RES:?}; SOCK=${SOCK:?}
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-probe5.XXXXXX)
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"; rm -f "$SOCK"; cd "$SCRATCH"
ctl() { "$CTL" --socket "$SOCK" "$@"; }
ans() { local i out; for i in 1 2 3 4 5; do ctl chat "/rftoolsbuilder $*" >/dev/null; ctl wait 8 >/dev/null; out=$(ctl messages 20 | jq -r '.chat[]' | grep -E "^(Card|Builder|Forced|not a builder|usage)" | head -1 || true); [ -n "$out" ] && { echo "$out"; return; }; done; echo "$out"; }
cleanup() { ctl quit >/dev/null 2>&1 || true; sleep 1; rm -rf "$SCRATCH" 2>/dev/null; echo "EXIT:$?"; }
trap cleanup EXIT
"$CTL" --socket "$SOCK" launch --game "$GAME" --world RFToolsBuilderProbe5 --seed 20261105 \
  --creative --offscreen --wait --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
ctl wait-ready 180 >/dev/null
PX=$(ctl state | jq -r '.player.position[0]|floor'); PZ=$(ctl state | jq -r '.player.position[2]|floor')
BX=$PX; BY=102; BZ=$((PZ+1))
ctl command "fill $((BX-4)) $((BY-3)) $((BZ-4)) $((BX+4)) $((BY+4)) $((BZ+4)) minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl setblock "$BX" $((BY-1)) "$BZ" minecraft:stone >/dev/null
ctl setblock "$BX" $((BY-2)) "$((BZ+3))" minecraft:stone >/dev/null
ctl command "setblock $BX $BY $BZ rftoolsbuilder:builder" >/dev/null; ctl wait 6 >/dev/null
ctl setblock "$BX" $((BY+1)) "$BZ" minecraft:chest >/dev/null
ctl command "clear @a" >/dev/null; ctl wait 4 >/dev/null
ctl give rftoolsbuilder:shape_card_def 1 >/dev/null; ctl wait 6 >/dev/null
ctl select rftoolsbuilder:shape_card_def >/dev/null; ctl wait 6 >/dev/null
echo "shape: $(ans shape box 3 3 3)"
echo "offset: $(ans offset 200 -152 200)"
echo "type: $(ans type quarry)"
ctl tp "$BX.5" "$((BY-1))" "$((BZ+3)).5" >/dev/null; ctl wait 6 >/dev/null
ctl look-at block "$BX" "$BY" "$BZ" south >/dev/null 2>&1; ctl wait 4 >/dev/null
echo "interact: $(ctl interact "$BX" "$BY" "$BZ" south 2>&1)"
ctl wait 12 >/dev/null
echo "status now: $(ans status $BX $BY $BZ)"
ctl wait 120 >/dev/null
echo "status after wait: $(ans status $BX $BY $BZ)"
echo "chest: $(ctl container $BX $((BY+1)) $BZ | jq -c '[.slots[]? | .count] | add // 0')"
echo "distant block: $(ctl block $((BX+199)) $((BY-153)) $((BZ+199)) 2>&1)"
