#!/usr/bin/env bash
# In-game check for RFTools Builder slice 1: the Builder and its shape card.
#
# Default run (offscreen, creative, a stone platform in the Overworld):
#   1. `rftoolsbuilder:builder` places, faces a side, and answers
#      `/rftoolsbuilder status` with `no shape card`;
#   2. CONTROL A — with no card, 60 ticks place nothing: the readout still says
#      `no shape card`.
#   3. `/rftoolsbuilder shape box 3 3 3` on a held card writes the card, and a
#      right click with it installs it into the Builder;
#   4. a chest above the Builder holds 128 stone; the Builder takes them from
#      there and fills its own 3×3×3 box — 26 cells, because the middle is the
#      Builder and `minecraft:chest` above it is not replaceable;
#   5. the status line reads `Box (Solid) 3x3x3 offset 0 0 0, 27 cells,
#      26 placed, 1 skipped, scan complete`;
#   6. the chest lost exactly 26 stone;
#   7. CONTROL B — a 5×5×5 HOLLOW box: a corner cell is stone and the middle of
#      an edge is air; the readout says `Box (Hollow) 5x5x5 ... 98 cells`;
#   8. sneaking with an empty hand takes the card back, and the status returns
#      to `no shape card`;
#   9. quitting and relaunching the same world keeps the placed blocks.
#
# COUNTERFACTUAL=1 puts a vanilla furnace where the Builder goes: the card's
# `/rftoolsbuilder shape` still writes, but `status` says `not a builder` and
# the click installs nothing, so no cell is ever built. That is the control
# that shows the placed volume is the port's own code.
#
# SELFTEST=1 keeps the scenario and asserts 26 SOLID cells for the 3×3×3 box
# (the box has 27), so the run MUST end in `FAIL:`: the count is not vacuous.
#
# The mod is one flat `.hmod` the launcher compiles into the game, so GAME must
# name a game the launcher built with it:
#   horizon-launcher native-build ports/rftoolsbuilder --instance <instance> --trust-native-code
# then `GAME=<that horizon-client> tools/e2e.sh`. There is no launcher at this
# pin on this machine, so this script has **not been run** (notes.md, gap 1).
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/rftoolsbuilder
SHARED=$CLONE/ports/_shared/bin/horizon-1d63e0a5
# A launcher-built `horizon-ctl` from the same build as GAME is the safe choice;
# the shared one speaks the same control protocol at this pin.
CTL=${CTL:-$SHARED/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/rftoolsbuilder-e2e.sock}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
SELFTEST=${SELFTEST:-0}

BUILDER=rftoolsbuilder:builder
CARD=rftoolsbuilder:shape_card_def

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME (build it with horizon-launcher native-build)" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rftoolsbuilder-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
say() { echo "== $* =="; }
block_name() { ctl block "$1" "$2" "$3" | jq -r '.block // "unknown"'; }
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // "unknown"'; }
block_facing() { block_state "$1" "$2" "$3" | sed -n 's/.*facing=\([a-z]*\).*/\1/p'; }
count_item() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'; }
action_bar() { ctl messages 10 | jq -r '.actionbar // ""'; }
# A native command goes through player chat (`perform_command`), not `command`.
mod() { ctl chat "/rftoolsbuilder $*" >/dev/null; ctl wait 8 >/dev/null; }
# The command's answer, from chat, without the colour prefix.
reply() { mod "$@"; ctl messages 10 | jq -r '.chat[]' | grep -m1 -E '^(Builder|[a-z0-9_.-]+:[a-z0-9_./-]+:)' || true; }
status() { reply status "$1" "$2" "$3"; }
use() { ctl interact "$1" "$2" "$3" "$4" >/dev/null; ctl wait 6 >/dev/null; }
# The number of `material` blocks in the cube between two corners.
volume() {
    local x1=$1 y1=$2 z1=$3 x2=$4 y2=$5 z2=$6 material=$7 n=0 x y z
    for ((x = x1; x <= x2; x++)); do
        for ((y = y1; y <= y2; y++)); do
            for ((z = z1; z <= z2; z++)); do
                [ "$(block_name "$x" "$y" "$z")" = "$material" ] && n=$((n + 1))
            done
        done
    done
    echo "$n"
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" 2>/dev/null || true
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() {
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world RFToolsBuilderE2E --seed 20261105 --creative --offscreen --wait \
        --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
    ctl wait-ready 180 >/dev/null
    if grep -q "content mods not loaded" "${SOCK}.log" 2>/dev/null; then
        grep "content mods not loaded" "${SOCK}.log" >&2
        fail "the content mod did not load"
    fi
}

say "launch (offscreen renderer, creative)"
launch

say "an Overworld platform at y=100, away from the terrain"
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
ctl command "fill $((PX-12)) 100 $((PZ-12)) $((PX+12)) 100 $((PZ+12)) minecraft:stone" >/dev/null
ctl command "fill $((PX-12)) 101 $((PZ-12)) $((PX+12)) 118 $((PZ+12)) minecraft:air" >/dev/null
ctl tp "$PX.5" 101 "$((PZ+4)).5" >/dev/null
ctl wait 20
ctl command "gamerule spawn_mobs false" >/dev/null
ctl command "kill @e[type=!minecraft:player]" >/dev/null
ctl wait 20
X=$PX
Y=102
Z1=$((PZ + 1))
ctl close >/dev/null 2>&1 || true
ctl look 180 0
ctl wait 6

if [ "$COUNTERFACTUAL" = 1 ]; then
    PLACE=minecraft:furnace
else
    PLACE=$BUILDER
fi

say "place $PLACE at $X $Y $Z1"
ctl setblock "$X" "$Y" $((Z1 - 1)) minecraft:stone >/dev/null
ctl give "$PLACE" 1 >/dev/null || fail "cannot give $PLACE"
ctl wait 6
ctl select 1 >/dev/null || true
ctl wait 6
ctl place "$X" "$Y" "$Z1" --item "$PLACE" >/dev/null || fail "cannot place $PLACE"
ctl wait 6
[ "$(block_name "$X" "$Y" "$Z1")" = "$PLACE" ] || fail "the placed block is $(block_name "$X" "$Y" "$Z1"), expected $PLACE"
facing=$(block_facing "$X" "$Y" "$Z1")
echo "placed: $(block_state "$X" "$Y" "$Z1")"
[ -n "$facing" ] || fail "the Builder has no facing property"
ctl look-at block "$X" "$Y" "$Z1" "$facing" >/dev/null || fail "cannot aim at the block"
ctl wait 6

if [ "$COUNTERFACTUAL" = 1 ]; then
    say "counterfactual: a vanilla furnace is not a builder"
    ctl give "$CARD" 1 >/dev/null
    ctl wait 6
    ctl select "$CARD" >/dev/null
    ctl wait 6
    mod shape box 3 3 3
    line=$(status "$X" "$Y" "$Z1")
    echo "status: $line"
    echo "$line" | grep -q 'not a builder' || fail "a furnace answered as a Builder: $line"
    ctl setblock "$X" $((Y + 1)) "$Z1" minecraft:chest >/dev/null
    ctl container "$X" $((Y + 1)) "$Z1" minecraft:stone 64 >/dev/null
    ctl wait 40
    built=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
    echo "stone cells around the furnace: $built"
    [ "$built" -eq 0 ] || fail "a furnace built $built cells"
    echo "PASS: the same commands build nothing on a furnace; the volume is the port's own code"
    exit 0
fi

say "a fresh Builder has no card"
line=$(status "$X" "$Y" "$Z1")
echo "status: $line"
echo "$line" | grep -q "Builder at $X $Y $Z1: no shape card" || fail "unexpected empty readout: $line"

say "CONTROL A: with no card nothing is built"
ctl setblock "$X" $((Y + 1)) "$Z1" minecraft:chest >/dev/null
ctl container "$X" $((Y + 1)) "$Z1" minecraft:stone 64 >/dev/null
ctl wait 60
built=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
echo "stone cells built without a card: $built"
[ "$built" -eq 0 ] || fail "the Builder built $built cells with no card"

say "a card is written by /rftoolsbuilder shape and installed by a click"
ctl command "clear @s" >/dev/null
ctl give "$CARD" 1 >/dev/null || fail "cannot give the card"
ctl wait 6
ctl select "$CARD" >/dev/null || fail "cannot select the card"
ctl wait 6
line=$(reply shape box 3 3 3)
echo "shape command: $line"
echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "the card was not written: $line"
use "$X" "$Y" "$Z1" "$facing"
bar=$(action_bar)
line=$(status "$X" "$Y" "$Z1")
echo "action bar: '$bar'; status: $line"
echo "$line" | grep -q "Box (Solid) 3x3x3 offset 0 0 0, 27 cells" || fail "the card is not in the Builder: $line"
[ "$(count_item "$CARD")" = 0 ] || fail "the card was not taken from the hand"

say "the Builder fills its 3×3×3 box from the chest above it"
ctl wait 80
built=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
line=$(status "$X" "$Y" "$Z1")
echo "status: $line; stone cells: $built"
# 27 cells: the Builder itself is one of them and the chest above it is not
# replaceable, so 26 are stone and 1 was skipped.
expected=26
[ "$SELFTEST" = 1 ] && expected=27
echo "$line" | grep -q "27 cells, 26 placed, 1 skipped, scan complete" || fail "unexpected progress: $line"
[ "$built" -eq "$expected" ] || fail "built $built cells, expected $expected"
left=$(ctl container "$X" $((Y + 1)) "$Z1" | jq -r '.counts["minecraft:stone"] // 0')
echo "stone left in the chest: $left"
[ "$left" -eq 38 ] || fail "the chest holds $left stone, expected 38 (64 - 26)"

say "CONTROL B: a hollow 5×5×5 box is its shell"
ctl command "clear @s" >/dev/null
ctl give "$CARD" 1 >/dev/null
ctl wait 6
ctl select "$CARD" >/dev/null
ctl wait 6
line=$(reply shape box 5 5 5 hollow)
echo "shape command: $line"
echo "$line" | grep -q "Box (Hollow) 5x5x5" || fail "the hollow card was not written: $line"
use "$X" "$Y" "$Z1" "$facing"
bar=$(action_bar)
ctl wait 200
line=$(status "$X" "$Y" "$Z1")
echo "action bar: '$bar'; status: $line"
echo "$line" | grep -q "Box (Hollow) 5x5x5 offset 0 0 0, 98 cells" || fail "unexpected hollow readout: $line"
corner=$(block_name $((X - 2)) $((Y + 2)) $((Z1 - 2)))
middle=$(block_name "$X" $((Y + 2)) "$Z1")
echo "corner: $corner; middle of an edge: $middle"
[ "$corner" = minecraft:stone ] || fail "the hollow box's corner is $corner"
[ "$middle" = minecraft:air ] || fail "the hollow box filled its middle ($middle)"

say "sneaking with an empty hand takes the card back"
ctl command "clear @s" >/dev/null
ctl wait 6
ctl key shift down >/dev/null
ctl wait 2
use "$X" "$Y" "$Z1" "$facing"
ctl key shift up >/dev/null
ctl wait 6
line=$(status "$X" "$Y" "$Z1")
taken=$(count_item "$CARD")
echo "status: $line; card back in the hand: $taken"
echo "$line" | grep -q "no shape card" || fail "the card was not taken out: $line"
[ "$taken" -eq 1 ] || fail "the card did not come back ($taken)"

say "quit and relaunch the same world: the placed blocks stay"
before=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
ctl quit >/dev/null 2>&1 || true
sleep 2
launch
after=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
echo "before: $before; after: $after"
[ "$before" -eq "$after" ] || fail "the relaunch changed the built volume: $before -> $after"

echo "PASS: the Builder fills its shape from the chest above it, hollow is its shell, and the volume survives a relaunch"
