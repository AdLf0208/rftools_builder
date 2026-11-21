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
#   4. a chest above the Builder holds 64 stone; the Builder takes them from
#      there and fills its own 3×3×3 box — 24 cells are built, because the
#      Builder itself, the chest above it and the pedestal under the box cannot
#      take a block;
#   5. the status line reads `Box (Solid) 3x3x3 offset 0 0 0, 27 cells,
#      24 placed, scan complete`;
#   6. the chest lost exactly 24 stone (64 -> 40);
#   7. CONTROL B — a 5×5×5 HOLLOW box: a corner cell is stone and the middle of
#      an edge is air; the readout says `Box (Hollow) 5x5x5 ... 98 cells`;
#   8. sneaking with an empty hand takes the card back, and the status returns
#      to `no shape card`;
#   9. quitting and relaunching the same world keeps the placed blocks;
#   10. the carried recipes: the shape card and the Builder are crafted in a real
#       crafting table (`tools/craft_check.py`), each with its declared id and
#       count, its distinctive ingredient consumed, the grid emptied, and a
#       wrong-ingredient control that leaves the result empty; the game log must
#       carry no `recipes: invalid ingredient` line (a rejected recipe prints one
#       and is dropped, so no craft can see it);
#   11. a box outside the world border builds 0 cells and the chest keeps every
#       block (the protected-cell rule);
#   12. the Builder's loot table drops it when broken with an iron pickaxe;
#   13. `/rftoolsbuilder reset` returns the held card to the jar's default solid
#       5x5x5 box, read back with `status` as 125 cells.
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
# name a game the launcher built with it. The Builder's recipe needs
# `rftoolsbase:machine_frame`, so the game carries the `rftoolsbase` package
# beside this one (upstream declares it a required dependency):
#   horizon-launcher native-build <folder holding both ports> --instance <i> --trust-native-code
# then `GAME=<that horizon-client> tools/e2e.sh`. Without rftoolsbase the frame
# cannot be given and `tools/craft_check.py` says which ingredient is missing
# rather than passing a half-check.
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
# `ctl launch` writes the game's log beside the socket, with the socket's own
# name and a `.log` suffix (not `.sock.log`): the loader's rejected-recipe lines
# live there, and a wrong path silently made the old check a no-op.
GAMELOG=${GAMELOG:-${SOCK%.sock}.log}
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
block_name() { ctl block "$1" "$2" "$3" | jq -r '.block // "unknown"' 2>/dev/null || echo unknown; }
block_state() { ctl block "$1" "$2" "$3" | jq -r '.state // "unknown"' 2>/dev/null || echo unknown; }
block_facing() { block_state "$1" "$2" "$3" | sed -n 's/.*facing=\([a-z]*\).*/\1/p' || true; }
count_item() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0' 2>/dev/null || echo 0; }
action_bar() { ctl messages 20 | jq -r '.actionbar // ""' 2>/dev/null || echo ""; }
# A native command goes through player chat (`perform_command`), not `command`.
mod() { ctl chat "/rftoolsbuilder $*" >/dev/null; ctl wait 8 >/dev/null; }
# The command's answer, from chat, without the colour prefix.
# The command's own answer: the newest chat line that reads as one (the buffer also holds the
# scenario's earlier replies, so the *last* match is the answer to this command).
reply() { mod "$@"; ctl messages 20 | jq -r '.chat[]' | grep -E '^(Builder|Card set to|Offset set to|Card reset|not a builder|Hold a shape card|Unknown|usage:|the last argument|[a-z0-9_.-]+:[a-z0-9_./-]+:)' | head -1 || true; }
# `ctl messages` lists the newest chat line first, so the **first** match is the newest. A readout
# can trail the tick the wait counts, so the command is repeated until a line newer than the one the
# buffer held when the call began appears: an earlier phase's readout can never be mistaken for this
# one's answer, and a readout that genuinely did not change is still returned.
status() {
    local i before now=""
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E "^Builder at $1 $2 $3:" | head -1 || true)
    for i in 1 2 3 4 5 6; do
        mod status "$1" "$2" "$3"
        ctl wait 10 >/dev/null
        now=$(ctl messages 20 | jq -r '.chat[]' | grep -E "^Builder at $1 $2 $3:" | head -1 || true)
        if [ -n "$now" ] && [ "$now" != "$before" ]; then
            echo "$now"
            return 0
        fi
    done
    echo "$now"
}
# A command's own answer, never a status readout, and asked again until it lands: a chat reply can
# trail the tick the wait counts (the readout lines start with "Builder", a card answer does not).
answer() {
    local i out=""
    for i in 1 2 3 4 5; do
        mod "$@"
        out=$(ctl messages 20 | jq -r '.chat[]' | grep -E '^(Card set to|Card offset set to|Card reset|Hold a shape card|Unknown|usage:|the last argument|not a builder)' | head -1 || true)
        if [ -n "$out" ]; then
            echo "$out"
            return 0
        fi
        ctl wait 5 >/dev/null
    done
    echo "$out"
}
# Put `item` in the main hand and prove it is there: `ctl select <item>` can land on a slot the
# click does not read, and a click with the wrong stack silently does nothing.
select_item() {
    ctl select "$1" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    local held
    held=$(ctl inventory | jq -r '.selected.item // .held.item // ""')
    if [ "$held" = "$1" ]; then
        return 0
    fi
    local slot
    slot=$(ctl inventory | jq -r --arg i "$1" '(.hotbar // []) | to_entries[] | select((.value.item // "") == $i) | (.key + 1)' | head -1)
    [ -n "$slot" ] || return 1
    ctl select "$slot" >/dev/null || return 1
    ctl wait 4 >/dev/null
    held=$(ctl inventory | jq -r '.selected.item // .held.item // ""')
    [ "$held" = "$1" ]
}

# Stand south of the Builder with a clear line to it: a finished pass has built stone in every cell
# around it, so the two layers the click looks through are cleared and the player is put outside the
# box (a Builder inside its own wall is only reachable through its GUI, which this slice has not).
clear_south() {
    ctl command "fill $((X - 2)) $((Y - 1)) $((Z1 + 1)) $((X + 2)) $((Y + 2)) $((Z1 + 3)) minecraft:air" >/dev/null 2>&1 || true
    ctl tp "$X.5" "$Y" "$((Z1 + 3)).5" >/dev/null 2>&1 || true
    ctl wait 10 >/dev/null
    ctl look 180 0 >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
}

# A fresh Builder at (X, Y, Z) with a chest above it and a card of the given shape installed by the
# click (the path the port supports: a card into an empty slot). Prints the facing.
new_builder() { # new_builder X Y Z SHAPE ARGS…
    local bx=$1 by=$2 bz=$3
    local card=$CARD
    shift 3
    # A clear site: the builder, its approach from the south and the box it will fill.
    ctl command "fill $((bx - 4)) $((by - 3)) $((bz - 4)) $((bx + 4)) $((by + 4)) $((bz + 4)) minecraft:air" >/dev/null 2>&1 || true
    ctl wait 6
    ctl setblock "$bx" $((by - 1)) "$bz" minecraft:stone >/dev/null
    ctl give "$BUILDER" 1 >/dev/null || fail "cannot give $BUILDER"
    ctl wait 6
    ctl select "$BUILDER" >/dev/null || fail "cannot select $BUILDER"
    ctl wait 6
    ctl place "$bx" "$by" "$bz" --item "$BUILDER" >/dev/null || fail "cannot place the Builder"
    ctl wait 6
    ctl setblock "$bx" $((by + 1)) "$bz" minecraft:chest >/dev/null
    # A chest holds 27 stacks, which is what a 98-cell shell needs.
    for _ in $(seq 1 27); do ctl container "$bx" $((by + 1)) "$bz" minecraft:stone 64 >/dev/null 2>&1 || true; done
    ctl wait 6
    ctl command "clear @a" >/dev/null
    ctl wait 4
    ctl give "$card" 1 >/dev/null || fail "cannot give the card"
    ctl wait 6
    ctl select "$card" >/dev/null || fail "cannot select the card"
    ctl wait 6
    local line
    line=$(answer shape "$@")
    echo "shape command: $line" >&2
    echo "$line" | grep -q "Card set to" || fail "the card was not written: $line"
    local facing
    facing=$(block_facing "$bx" "$by" "$bz")
    [ -n "$facing" ] || facing=south
    ctl look-at block "$bx" "$by" "$bz" "$facing" >/dev/null
    ctl wait 6
    echo "click reply: $(ctl interact "$bx" "$by" "$bz" "$facing" 2>&1 | head -2)" >&2
    ctl wait 10 >/dev/null
    ctl messages 20 | jq -r '.actionbar // ""'
}

# A click: the assertions after each one read the result, so a click the game reports as a failed
# interaction is not fatal here (the readout, the slot and the volume decide).
use() { ctl interact "$1" "$2" "$3" "$4" >/dev/null 2>&1 || true; ctl wait 6 >/dev/null; }
# Sneak-right-click with an empty hand: takes the installed card back out.
take_out() {
    ctl command "clear @s" >/dev/null
    ctl wait 6 >/dev/null
    ctl key shift down >/dev/null
    ctl wait 2 >/dev/null
    use "$1" "$2" "$3" "$4"
    ctl key shift up >/dev/null
    ctl wait 6 >/dev/null
}
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

# The stone in the chest above the Builder (`ctl container` lists slots, not a counts map).
chest_stone() {
    ctl container "$X" $((Y + 1)) "$Z1" \
        | jq -r '[.slots[]? | select(.item == "minecraft:stone") | .count] | add // 0'
}

# The amount of `item` in the container at a position.
chest_count() { # x y z item
    ctl container "$1" "$2" "$3" \
        | jq -r --arg i "$4" '[.slots[]? | select(.item == $i) | .count] | add // 0'
}

# The stone in the 3x3x3 box around the Builder that the *port* placed: the scenario's own
# pedestal block sits inside that box, so it is subtracted.
built_in_shape() {
    local n
    n=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
    if [ "$(block_name "$X" "$Y" $((Z1 - 1)))" = minecraft:stone ]; then
        n=$((n - 1))
    fi
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
    if grep -q "content mods not loaded" "${GAMELOG}" 2>/dev/null; then
        grep "content mods not loaded" "${GAMELOG}" >&2
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
ctl give "$PLACE" 1 >/dev/null || fail "cannot give $PLACE: this game does not carry the package"
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
    echo "$line" | grep -q 'minecraft:furnace' && fail "the refusal names the block there: $line"
    ctl setblock "$X" $((Y + 1)) "$Z1" minecraft:chest >/dev/null
    ctl container "$X" $((Y + 1)) "$Z1" minecraft:stone 64 >/dev/null
    ctl wait 40
    built=$(volume $((X - 1)) $((Y - 1)) $((Z1 - 1)) $((X + 1)) $((Y + 1)) $((Z1 + 1)) minecraft:stone)
    echo "stone cells around the furnace: $built"
    [ "$built" -eq 0 ] || fail "a furnace built $built cells"
    echo "PASS: the same commands build nothing on a furnace; the volume is the port's own code"
    exit 0
fi

say "the carried recipes craft in a real crafting table"
CX=$((PX + 8))
CY=101
CZ=$((PZ - 8))
python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" "$CX" "$CY" "$CZ" || fail "a carried recipe did not craft"
# The loader's own verdict: a recipe it rejects prints this line and is dropped,
# so a green craft alone would not see the other recipe. Zero lines is the claim.
if grep -q "recipes: invalid ingredient" "${GAMELOG}" 2>/dev/null; then
    grep "recipes: invalid ingredient" "${GAMELOG}" | head -5 >&2
    fail "the game rejected a carried recipe ingredient"
fi
echo "recipe log: no 'invalid ingredient' line"
# Back to the arena beside the Builder for the clicks that follow.
ctl tp "$PX.5" 101 "$((PZ + 4)).5" >/dev/null
ctl wait 10 >/dev/null

say "a fresh Builder has no card"
line=$(status "$X" "$Y" "$Z1")
echo "status: $line"
echo "$line" | grep -q "Builder at $X $Y $Z1: no shape card" || fail "unexpected empty readout: $line"

say "CONTROL A: with no card nothing is built"
ctl setblock "$X" $((Y + 1)) "$Z1" minecraft:chest >/dev/null
ctl container "$X" $((Y + 1)) "$Z1" minecraft:stone 64 >/dev/null
ctl wait 60
built=$(built_in_shape)
echo "stone cells built without a card: $built"
[ "$built" -eq 0 ] || fail "the Builder built $built cells with no card"

say "a card is written by /rftoolsbuilder shape and installed by a click"
ctl command "clear @s" >/dev/null
ctl give "$CARD" 1 >/dev/null || fail "cannot give the card"
ctl wait 6
ctl select "$CARD" >/dev/null || fail "cannot select the card"
ctl wait 6
line=$(answer shape box 3 3 3)
echo "shape command: $line"
echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "the card was not written: $line"
use "$X" "$Y" "$Z1" "$facing"
bar=$(action_bar)
line=$(status "$X" "$Y" "$Z1")
echo "action bar: '$bar'; status: $line"
if ! echo "$line" | grep -q "Box (Solid) 3x3x3 offset 0 0 0, 27 cells"; then
    echo "chat buffer: $(ctl messages 20 | jq -c '.chat')" >&2
    echo "card slot: $(ctl container "$X" "$Y" "$Z1" | jq -c '[.slots[]? | .item]')" >&2
    fail "the card is not in the Builder: $line"
fi
[ "$(count_item "$CARD")" = 0 ] || fail "the card was not taken from the hand"

say "the Builder fills its 3×3×3 box from the chest above it"
ctl wait 80
built=$(built_in_shape)
line=$(status "$X" "$Y" "$Z1")
echo "status: $line; stone cells: $built"
# The 3x3x3 box holds 27 cells. Three of them cannot take a block: the Builder itself, the chest
# above it and the scenario's own pedestal under the box's corner. So 24 are stone, 3 are skipped,
# and the three numbers must add up to the shape — the invariant, not just the three values.
expected=24
[ "$SELFTEST" = 1 ] && expected=25
# The finished readout prints the placed count and "scan complete" (the skipped count is only named
# while a pass is still running), so the world count and the chest are what pin the three cells.
echo "$line" | grep -q "27 cells, 24 placed, scan complete" || fail "unexpected progress: $line"
[ "$built" -eq "$expected" ] || fail "built $built cells, expected $expected"
[ "$((built + 3))" -eq 27 ] || fail "the box's 27 cells are not 24 stone + 3 that cannot take one"
left=$(chest_stone)
echo "stone left in the chest: $left"
[ "$left" -eq 40 ] || fail "the chest holds $left stone, expected 40 (64 - 24)"

say "CONTROL B: a hollow 5×5×5 box is its shell"
# The same Builder, a second card (the slot swaps): a hollow 5x5x5 is a 98-cell shell.
# A full chest, so the whole 98-cell shell can be paid for (the solid pass left 40).
for _ in $(seq 1 27); do ctl container "$X" $((Y + 1)) "$Z1" minecraft:stone 64 >/dev/null 2>&1 || true; done
chest_full=$(chest_stone)
ctl command "clear @s" >/dev/null
ctl give "$CARD" 1 >/dev/null || fail "cannot give the hollow card"
ctl wait 6
ctl select "$CARD" >/dev/null || fail "cannot select the hollow card"
ctl wait 6
line=$(answer shape box 5 5 5 hollow)
echo "shape command: $line"
echo "$line" | grep -q "Box (Hollow) 5x5x5" || fail "the hollow card was not written: $line"
# Lift the box off the arena floor: the platform at y=100 would be the shell's bottom face (25
# stone cells, already there, correctly skipped), and the proof below needs all 98 placeable.
line=$(answer offset 0 2 0)
echo "offset command: $line"
echo "$line" | grep -q "Card offset set to 0 2 0" || fail "the offset was not written: $line"
clear_south
ctl look-at block "$X" "$Y" "$Z1" "$facing" >/dev/null 2>&1 || true
ctl wait 6
use "$X" "$Y" "$Z1" "$facing"
bar=$(action_bar)
ctl wait 200
line=$(status "$X" "$Y" "$Z1")
echo "action bar: '$bar'; status: $line"
echo "$line" | grep -q "Box (Hollow) 5x5x5 offset 0 2 0, 98 cells" || fail "unexpected hollow readout: $line"
# The shape is 98 cells where a solid 5x5x5 would be 125. Six of the shell's cells are the *first*
# pass's own stone, at the bottom layer of the lifted box — a Builder never replaces what is already
# there, so 92 are built and the chest must lose exactly 92. (Measured with the box NOT lifted: 73 =
# 98 - 25, the arena platform's 25 stone cells forming the shell's bottom face.) A walk that
# under-visits lands below 92 and fails here.
echo "$line" | grep -q "98 cells, 92 placed, scan complete" || fail "the shell is not complete: $line"
chest_after=$(chest_stone)
[ "$((chest_full - chest_after))" -eq 92 ] \
    || fail "the shell took $((chest_full - chest_after)) blocks, expected 92"
corner=$(block_name $((X - 2)) $((Y + 4)) $((Z1 - 2)))
edge=$(block_name $((X + 2)) $((Y + 4)) "$Z1")
echo "corner: $corner; edge: $edge; chest $chest_full -> $chest_after"
[ "$corner" = minecraft:stone ] || fail "the hollow box's corner is $corner"
[ "$edge" = minecraft:stone ] || fail "the hollow box left its shell open ($edge)"

say "sneaking with an empty hand takes the card back"
# An empty hand (the whole inventory), a clear line to the Builder and a sneak right-click.
ctl command "clear @a" >/dev/null
ctl wait 6 >/dev/null
clear_south
ctl key shift down >/dev/null 2>&1 || true
ctl wait 4 >/dev/null
use "$X" "$Y" "$Z1" "$facing"
ctl key shift up >/dev/null 2>&1 || true
ctl wait 10 >/dev/null
taken=$(count_item "$CARD")
line=$(status "$X" "$Y" "$Z1")
echo "status: $line; cards in the inventory: $taken"
echo "$line" | grep -q "no shape card" || fail "the card was not taken out: $line"
[ "$taken" -eq 1 ] || fail "the card did not come back ($taken)"

say "quit and relaunch the same world: the placed blocks stay"
before=$(built_in_shape)
ctl quit >/dev/null 2>&1 || true
sleep 2
launch
after=$(built_in_shape)
echo "before: $before; after: $after"
[ "$before" -eq "$after" ] || fail "the relaunch changed the built volume: $before -> $after"

say "a cell outside the world border is skipped and costs nothing"
# A second Builder on the platform, a fresh card whose 3x3x3 box sits eight blocks away, and a world
# border that encloses the Builder, its chest and the player but not that box: every cell is
# protected, so the pass places nothing and the chest keeps every block it held.
BX=$((X + 6)); BY=102; BZ=$((Z1 + 6))
ctl command "fill $((BX - 4)) 101 $((BZ - 4)) $((BX + 4)) $((BY + 4)) $((BZ + 4)) minecraft:air" >/dev/null
ctl wait 6
ctl setblock "$BX" $((BY - 1)) "$BZ" minecraft:stone >/dev/null
ctl give "$BUILDER" 1 >/dev/null || fail "cannot give a second Builder"
ctl wait 6
ctl select "$BUILDER" >/dev/null
ctl wait 4
ctl tp "$((BX - 2)).5" "$BY" "$((BZ + 1)).5" >/dev/null
ctl wait 10
ctl place "$BX" "$BY" "$BZ" --item "$BUILDER" >/dev/null || fail "cannot place the second Builder"
[ "$(block_name "$BX" "$BY" "$BZ")" = "$BUILDER" ] || fail "the second Builder is not there"
ctl setblock "$BX" $((BY + 1)) "$BZ" minecraft:chest >/dev/null
for _ in $(seq 1 4); do ctl container "$BX" $((BY + 1)) "$BZ" minecraft:stone 64 >/dev/null 2>&1 || true; done
guarded=$(chest_count "$BX" $((BY + 1)) "$BZ" minecraft:stone)
ctl command "clear @s" >/dev/null
ctl wait 4
ctl give "$CARD" 1 >/dev/null || fail "cannot give the guarded card"
ctl wait 6
ctl select "$CARD" >/dev/null
ctl wait 4
answer shape box 3 3 3 >/dev/null
answer offset 8 0 0 >/dev/null
ctl command "worldborder center $BX $BZ" >/dev/null
ctl command "worldborder set 8" >/dev/null
ctl wait 20
ctl tp "$((BX + 1)).5" "$BY" "$((BZ + 1)).5" >/dev/null
ctl wait 10
bf=$(block_facing "$BX" "$BY" "$BZ"); [ -n "$bf" ] || bf=south
ctl look-at block "$BX" "$BY" "$BZ" "$bf" >/dev/null 2>&1 || true
ctl wait 6
use "$BX" "$BY" "$BZ" "$bf"
ctl wait 120
guarded_line=$(status "$BX" "$BY" "$BZ")
guarded_after=$(chest_count "$BX" $((BY + 1)) "$BZ" minecraft:stone)
echo "status: $guarded_line; chest $guarded -> $guarded_after"
echo "$guarded_line" | grep -q "27 cells, 0 placed" || fail "the guarded box was not held at 0 placed: $guarded_line"
[ "$guarded" -eq "$guarded_after" ] || fail "the guarded pass took $((guarded - guarded_after)) blocks"
ctl command "worldborder center 0 0" >/dev/null
ctl command "worldborder set 60000000" >/dev/null
ctl wait 10

say "the Builder's loot table drops the Builder"
# `ctl break` mines the way a player does: survival (creative's instant break is not on that path)
# and far from the world spawn, whose protection refuses a non-op player's break. So the block is
# set in The End, where the campaign's item-count scenarios go because it has no ambient drops, and
# the drop needs the correct tool (`needs_iron_tool`) — an iron pickaxe.
ctl command "execute in minecraft:the_end run tp @s 3.5 100 5.5" >/dev/null
ctl wait 20
echo "fill reply: $(ctl command "execute in minecraft:the_end run fill 0 63 0 6 63 6 minecraft:stone" 2>&1)"
echo "setblock reply: $(ctl command "execute in minecraft:the_end run setblock 3 64 3 $BUILDER" 2>&1)"
echo "tp reply: $(ctl command "execute in minecraft:the_end run tp @s 3.5 64 5.5" 2>&1)"
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 20
echo "dimension: $(ctl state | jq -c '{dimension: (.dimension // .world.dimension // "?"), pos: .player.position}')"
echo "end block: $(block_name 3 64 3)"
[ "$(block_name 3 64 3)" = "$BUILDER" ] || fail "the Builder to break is not in The End"
ctl give minecraft:iron_pickaxe 1 >/dev/null || fail "cannot give the iron pickaxe"
ctl wait 6
ctl select minecraft:iron_pickaxe >/dev/null || fail "cannot hold the iron pickaxe"
ctl wait 4
ctl look-at block 3 64 3 >/dev/null 2>&1 || true
ctl wait 6
if ! loot_break=$(ctl break 3 64 3 --timeout 150 2>&1); then
    echo "break reply: $loot_break; state: $(ctl state | jq -c '{pos: .player.position, target: .crosshair}')"
    fail "cannot break the Builder"
fi
echo "break reply: $loot_break"
# The break reply is the loot table's own verdict (`drops`); then the drop is picked up by walking
# onto it, so the item's presence in the inventory is asserted too.
loot_drop=$(jq -r --arg i "$BUILDER" '.drops[$i] // 0' <<<"$loot_break" 2>/dev/null || echo 0)
echo "loot table drops: $loot_drop"
[ "$loot_drop" -ge 1 ] || fail "the Builder's loot table dropped $loot_drop of it: $loot_break"
ctl tp 3.5 64 3.5 >/dev/null
ctl wait 30
loot_block=$(block_name 3 64 3)
loot=$(count_item "$BUILDER")
echo "after the break: $loot_block at 3 64 3; builders in the inventory: $loot"
[ "$loot_block" = minecraft:air ] || fail "the broken Builder is still $loot_block"
[ "$loot" -eq 1 ] || fail "the Builder's loot table dropped $loot of it, expected 1"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl command "execute in minecraft:overworld run tp @s $((X + 0)).5 $Y $((Z1 + 3)).5" >/dev/null
ctl wait 20

say "/rftoolsbuilder reset returns the held card to the default solid 5x5x5 box"
ctl command "clear @s" >/dev/null
ctl wait 6
ctl give "$CARD" 1 >/dev/null || fail "cannot give the card for reset"
ctl wait 6
ctl select "$CARD" >/dev/null
ctl wait 4
reset_line=$(answer reset)
echo "reset: $reset_line"
# The reply is the command's readout of the card it just wrote: `ShapeCardData.DEFAULT`, a solid
# 5x5x5 box at offset 0 0 0 (125 cells). The install path itself is proven by the 3x3x3 phase above.
echo "$reset_line" | grep -q "Card reset to the jar's default: Box (Solid) 5x5x5 offset 0 0 0" \
    || fail "reset did not write the jar's default card: $reset_line"
cards=$(count_item "$CARD")
echo "cards in the inventory after reset: $cards"
[ "$cards" -eq 1 ] || fail "the reset consumed the card ($cards left)"

echo "PASS: both carried recipes craft in a real crafting table (the shape card from 4 paper, 2 bricks, 1 iron ingot and 2 redstone; the Builder from 4 bricks, an ender pearl, 3 redstone and a machine frame) with their ingredients consumed and their wrong-ingredient controls empty, the game log has no rejected ingredient, the Builder fills its 3x3x3 box from the chest above it (24 placed, 64 -> 40), a hollow 5x5x5 is a 98-cell shell (92 placed against a full chest), a box outside the world border builds 0 and spends nothing, the Builder's loot table drops it, the card goes in, comes back out, resets to the 5x5x5 default, and the placed volume survives a relaunch"
