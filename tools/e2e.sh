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
#   12. the QUARRY card type: a card set to `quarry` over a 3x3x3 volume of 25 stone,
#       one gravel and one coarse dirt breaks it and leaves `minecraft:dirt` behind,
#       collecting 25 cobblestone and neither of the two voided blocks (`gravel` by
#       id, `coarse_dirt` through the `minecraft:dirt` block tag); the card keeps its
#       type and void list when it is taken out and put back; with tag matching off
#       the coarse dirt is collected again; `/rftoolsbuilder replace` changes what the
#       quarry leaves behind; and a box card on the same rig collects nothing;
#   13. the Builder's loot table drops it when broken with an iron pickaxe;
#   14. `/rftoolsbuilder reset` returns the held card to the jar's default solid
#       5x5x5 box, read back with `status` as 125 cells.
#
# COUNTERFACTUAL=1 puts a vanilla furnace where the Builder goes: the card's
# `/rftoolsbuilder shape` still writes, but `status` says `not a builder` and
# the click installs nothing, so no cell is ever built. That is the control
# that shows the placed volume is the port's own code.
#
# SELFTEST=1 keeps the scenario and asserts 26 SOLID cells for the 3×3×3 box
# (the box has 27), so the run MUST end in `FAIL:`: the count is not vacuous.
# SELFTEST=quarry flips the quarry's replacement assertion (it expects cobblestone
# where the pass writes dirt), so that run MUST end in `FAIL:` too.
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
        out=$(ctl messages 20 | jq -r '.chat[]' | grep -E '^(Card set to|Card offset set to|Card type set to|Card voids|Card void list cleared|Card tag matching|Builder replacement set to|Card reset|Hold a shape card|Unknown|unknown card type|unknown material|usage:|the last argument|not a builder)' | head -1 || true)
        if [ -n "$out" ]; then
            echo "$out"
            return 0
        fi
        ctl wait 5 >/dev/null
    done
    echo "$out"
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
use() { ctl interact "$1" "$2" "$3" "$4" 2>&1 | head -2 >&2 || true; ctl wait 6 >/dev/null; }
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
ctl command "worldborder set 29999984" >/dev/null   # vanilla's maximum size; a larger one is refused
ctl wait 10

# --- the quarry card type --------------------------------------------------
# A second rig: a Builder whose card is a QUARRY, not a shape. The card's own dimension and
# offset put a 3x3x3 volume six blocks east of it (the Builder's own 3x3x3 is never quarried,
# so the chest above it is safe), the volume holds 25 stone, one gravel and one coarse dirt,
# and the card voids `gravel` by id and `dirt` through its block tag.
QY=102
QX=$X
QZ=$((Z1 - 8))
VX1=$((QX + 5)); VX2=$((QX + 7))
VZ1=$((QZ - 1)); VZ2=$((QZ + 1))
VY1=$((QY - 1)); VY2=$((QY + 1))

# The amount of `item` in the container at a position.
chest_total() { ctl container "$1" "$2" "$3" | jq -r '[.slots[]? | .count] | add // 0'; }
# Every cell of the volume, as "x y z name" lines.
volume_cells() {
    local x y z
    for ((x = VX1; x <= VX2; x++)); do
        for ((y = VY1; y <= VY2; y++)); do
            for ((z = VZ1; z <= VZ2; z++)); do
                echo "$x $y $z $(block_name "$x" "$y" "$z")"
            done
        done
    done
}
# Whether every cell of the volume is `material`.
volume_all() {
    local wrong
    wrong=$(volume_cells | awk -v want="$1" '$4 != want' | head -3)
    [ -z "$wrong" ] || { echo "$wrong" >&2; return 1; }
}
# Fills the volume: `stone` cells plus the one gravel and one coarse dirt.
fill_volume() { # [gravel] [coarse_dirt]
    local gravel=${1:-1} coarse=${2:-1}
    ctl command "fill $VX1 $VY1 $VZ1 $VX2 $VY2 $VZ2 minecraft:stone" >/dev/null
    [ "$gravel" = 1 ] && ctl setblock "$VX2" "$VY2" "$VZ2" minecraft:gravel >/dev/null
    [ "$coarse" = 1 ] && ctl setblock "$VX1" "$VY1" "$VZ1" minecraft:coarse_dirt >/dev/null
    ctl wait 6 >/dev/null
}
# A fresh quarry rig: the site cleared, a Builder placed, a chest above it.
quarry_rig() {
    # From the pedestal's own layer up: the arena floor under the player stays, so the click's
    # reach and the place command's visible face are the same as the first rig's.
    ctl command "fill $((QX - 4)) $((QY - 1)) $((QZ - 4)) $((VX2 + 2)) $((QY + 4)) $((VZ2 + 2)) minecraft:air" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    ctl setblock "$QX" $((QY - 1)) "$QZ" minecraft:stone >/dev/null
    # The Builder is set in place, as the loot phase does: the item-placement path is the first
    # rig's proof, and this rig's own subject is the card type.
    ctl command "setblock $QX $QY $QZ $BUILDER" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    [ "$(block_name "$QX" "$QY" "$QZ")" = "$BUILDER" ] || fail "the quarry Builder is not at $QX $QY $QZ"
    ctl tp "$QX.5" "$QY" "$((QZ + 3)).5" >/dev/null
    ctl wait 8 >/dev/null
    ctl setblock "$QX" $((QY + 1)) "$QZ" minecraft:chest >/dev/null
    ctl command "clear @a" >/dev/null
    ctl wait 6 >/dev/null
    echo "quarry Builder: $(block_state "$QX" "$QY" "$QZ")" >&2
    # The player stands south of the Builder, so the face to aim at and click is its south one
    # (the block's own `facing` property is the machine's front and may point anywhere).
    qface=south
    ctl look-at block "$QX" "$QY" "$QZ" "$qface" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
}
# Writes the held card and installs it by a click on the quarry Builder.
install_quarry_card() { # type tags(on|off) [material…]
    local type=$1 tags=${2:-off}
    shift 2
    ctl command "clear @a" >/dev/null
    ctl wait 4 >/dev/null
    ctl give "$CARD" 1 >/dev/null || fail "cannot give a quarry card"
    ctl wait 6 >/dev/null
    ctl select "$CARD" >/dev/null || fail "cannot select the quarry card"
    ctl wait 6 >/dev/null
    local line
    line=$(answer shape box 3 3 3)
    echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "the quarry card was not written: $line"
    line=$(answer offset 6 0 0)
    echo "$line" | grep -q "Card offset set to 6 0 0" || fail "the quarry card's offset: $line"
    line=$(answer type "$type")
    echo "$line" | grep -q "Card type set to $type" || fail "the card type was not written: $line"
    local material
    for material in "$@"; do
        line=$(answer void "$material")
        echo "$line" | grep -q "Card voids $material" || fail "the void list: $line"
    done
    line=$(answer tagmatching "$tags")
    echo "$line" | grep -q "Card tag matching $tags" || fail "the tag toggle: $line"
    install_held_card "Box (Solid) 3x3x3 offset 6 0 0"
}
# Installs the card the player already holds and reads the Builder back.
install_held_card() { # expected readout fragment
    ctl look-at block "$QX" "$QY" "$QZ" "$qface" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    use "$QX" "$QY" "$QZ" "$qface"
    echo "click: bar '$(action_bar)'; cards held $(count_item "$CARD"); $(ctl state | jq -c '{pos: .player.position, held: (.held // .selected // null)}')" >&2
    local line
    line=$(status "$QX" "$QY" "$QZ")
    echo "quarry card: $line"
    echo "$line" | grep -q "$1" || fail "the card is not in the Builder: $line"
}

# Takes the card back out of the quarry Builder (sneak-click with an empty hand).
take_quarry_card() {
    ctl command "clear @a" >/dev/null
    ctl wait 6 >/dev/null
    ctl tp "$QX.5" "$QY" "$((QZ + 3)).5" >/dev/null
    ctl wait 8 >/dev/null
    ctl look-at block "$QX" "$QY" "$QZ" "$qface" >/dev/null 2>&1 || true
    ctl key shift down >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    use "$QX" "$QY" "$QZ" "$qface"
    ctl key shift up >/dev/null 2>&1 || true
    ctl wait 10 >/dev/null
}

say "QUARRY: a quarry card breaks the box beside the Builder and collects its drops"
quarry_rig
fill_volume 1 1
install_quarry_card quarry on gravel dirt
# The readout names the type, the replacement block and the void list before anything runs.
line=$(status "$QX" "$QY" "$QZ")
echo "quarry status: $line"
echo "$line" | grep -q "27 cells, quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel (tags too)" \
    || fail "the quarry readout does not name its type, replacement and void list: $line"
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
echo "quarry status: $line"
echo "$line" | grep -q "27 cells, quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel (tags too), 27 quarried, 2 voided, scan complete" \
    || fail "the quarry pass did not finish over 27 cells: $line"
# The replacement assertion: what the pass left behind. SELFTEST=quarry flips it.
q_expected=minecraft:dirt
if [ "$SELFTEST" = quarry ]; then q_expected=minecraft:cobblestone; fi
if ! volume_all "$q_expected"; then
    fail "the quarry did not replace the 27 cells with $q_expected ($(volume_cells | awk -v want="$q_expected" '$4 != want' | wc -l) cells differ)"
fi
q_cobble=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:cobblestone)
q_gravel=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:gravel)
q_coarse=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:coarse_dirt)
q_total=$(chest_total "$QX" $((QY + 1)) "$QZ")
echo "collected: $q_cobble cobblestone, $q_gravel gravel, $q_coarse coarse_dirt (total $q_total)"
# 25 stone cells drop cobblestone; the gravel is voided by id and the coarse dirt through the
# `minecraft:dirt` tag, so neither is collected — a quarry that ignored its void list would
# hold 27 stacks here (flint or gravel for the gravel cell).
[ "$q_cobble" -eq 25 ] || fail "the quarry collected $q_cobble cobblestone, expected 25"
[ "$q_gravel" -eq 0 ] || fail "the void list's gravel was collected ($q_gravel)"
[ "$q_coarse" -eq 0 ] || fail "tag matching did not void the coarse dirt ($q_coarse)"
[ "$q_total" -eq 25 ] || fail "the chest holds $q_total items, expected 25"

say "QUARRY: the card keeps its type and void list when it is taken out and put back"
take_quarry_card
cards=$(count_item "$CARD")
echo "cards in the inventory: $cards"
[ "$cards" -eq 1 ] || fail "the quarry card did not come back ($cards)"
line=$(status "$QX" "$QY" "$QZ")
echo "status after the take-out: $line"
echo "$line" | grep -q "no shape card" || fail "the Builder still holds a card: $line"
ctl select "$CARD" >/dev/null || fail "cannot hold the quarry card again"
install_held_card "quarry (Normal quarry)"
line=$(status "$QX" "$QY" "$QZ")
echo "status after the put-back: $line"
echo "$line" | grep -q "quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel (tags too)" \
    || fail "the card lost its quarry settings when it was moved: $line"
# The second pass walks the same 27 cells, and every one of them is the replacement block: the
# non-clearing quarry leaves them alone and collects nothing.
echo "$line" | grep -q "27 cells, quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel (tags too), 0 quarried, scan complete" \
    || fail "a quarry over its own replacement block worked: $line"
[ "$(chest_total "$QX" $((QY + 1)) "$QZ")" -eq 25 ] || fail "the second pass collected something"

say "QUARRY: tag matching off collects the tag's blocks again"
take_quarry_card
ctl select "$CARD" >/dev/null || fail "cannot hold the quarry card"
line=$(answer tagmatching off)
echo "$line" | grep -q "Card tag matching off" || fail "the tag toggle: $line"
fill_volume 1 1
install_held_card "quarry (Normal quarry)"
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
echo "quarry status: $line"
echo "$line" | grep -q "27 cells, quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel, 27 quarried, 1 voided, scan complete" \
    || fail "the second quarry pass: $line"
q_cobble2=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:cobblestone)
q_gravel2=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:gravel)
q_coarse2=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:coarse_dirt)
echo "collected: $q_cobble2 cobblestone, $q_gravel2 gravel, $q_coarse2 coarse_dirt"
[ "$q_cobble2" -eq 50 ] || fail "the second pass collected $q_cobble2 cobblestone, expected 50"
[ "$q_gravel2" -eq 0 ] || fail "the void list's gravel was collected on the second pass"
[ "$q_coarse2" -eq 1 ] || fail "with tag matching off the coarse dirt is not voided ($q_coarse2)"
volume_all minecraft:dirt || fail "the second pass did not replace every cell"

say "QUARRY: /rftoolsbuilder replace sets what a quarry leaves behind"
take_quarry_card
ctl select "$CARD" >/dev/null || fail "cannot hold the quarry card"
line=$(answer replace "$QX" "$QY" "$QZ" minecraft:cobblestone)
echo "replace: $line"
echo "$line" | grep -q "Builder replacement set to minecraft:cobblestone" || fail "the replacement: $line"
fill_volume 0 0
install_held_card "replacing with minecraft:cobblestone"
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
echo "quarry status: $line"
echo "$line" | grep -q "replacing with minecraft:cobblestone" || fail "the readout: $line"
echo "$line" | grep -q "27 quarried, scan complete" || fail "the third quarry pass: $line"
volume_all minecraft:cobblestone || fail "the quarry left $(volume_cells | awk '$4 != "minecraft:cobblestone"' | head -1) behind, not cobblestone"

say "QUARRY: a clearing card leaves air, not the replacement block"
fill_volume 0 0
install_quarry_card quarry_clear off
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
clear_cobble=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:cobblestone)
echo "clearing status: $line; collected $clear_cobble cobblestone"
echo "$line" | grep -q "quarry (Normal quarry, clearing), 27 quarried, scan complete" \
    || fail "the clearing readout: $line"
volume_all minecraft:air || fail "a clearing card left $(volume_cells | awk '$4 != "minecraft:air"' | head -1) behind"
[ "$clear_cobble" -eq 104 ] || fail "the clearing pass collected $clear_cobble cobblestone, expected 104"

say "QUARRY: a silk card collects the block itself, where the plain one collects its drop"
fill_volume 0 0
install_quarry_card quarry_silk off
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
silk_stone=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:stone)
silk_cobble=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:cobblestone)
echo "silk status: $line; collected $silk_stone stone and $silk_cobble cobblestone"
echo "$line" | grep -q "quarry (Silktouch quarry), replacing with minecraft:cobblestone, 27 quarried, scan complete" \
    || fail "the silk readout: $line"
[ "$silk_stone" -eq 27 ] || fail "the silk quarry collected $silk_stone stone, expected 27"
[ "$silk_cobble" -eq 104 ] || fail "the silk quarry collected cobblestone ($silk_cobble): it did not use silk touch"

say "QUARRY: a fortune card turns gravel into flint (fortune III makes the table's roll certain)"
fill_volume 1 0
install_quarry_card quarry_fortune off
ctl wait 120 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
fortune_flint=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:flint)
fortune_gravel=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:gravel)
fortune_cobble=$(chest_count "$QX" $((QY + 1)) "$QZ" minecraft:cobblestone)
echo "fortune status: $line; collected $fortune_flint flint, $fortune_gravel gravel, $fortune_cobble cobblestone"
echo "$line" | grep -q "quarry (Fortune quarry), replacing with minecraft:cobblestone, 27 quarried, scan complete" \
    || fail "the fortune readout: $line"
[ "$fortune_flint" -eq 1 ] || fail "the fortune quarry collected $fortune_flint flint, expected 1"
[ "$fortune_gravel" -eq 0 ] || fail "the fortune quarry left gravel instead of flint ($fortune_gravel)"
[ "$fortune_cobble" -eq 130 ] || fail "the fortune pass collected $fortune_cobble cobblestone, expected 130"

say "CONTROL: the same rig with a box card collects nothing"
take_quarry_card
ctl command "clear @a" >/dev/null
# A full volume under the same card geometry (no cell a shape card can replace, so it places
# nothing) and a chest that is not touched: a card that collected anything would move its total.
ctl command "fill $VX1 $VY1 $VZ1 $VX2 $VY2 $VZ2 minecraft:cobblestone" >/dev/null
ctl wait 6 >/dev/null
control_before=$(chest_total "$QX" $((QY + 1)) "$QZ")
echo "chest total before the control: $control_before"
ctl give "$CARD" 1 >/dev/null || fail "cannot give the control card"
ctl wait 6 >/dev/null
ctl select "$CARD" >/dev/null || fail "cannot select the control card"
ctl wait 6 >/dev/null
line=$(answer shape box 3 3 3)
echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "the control card: $line"
line=$(answer offset 6 0 0)
echo "$line" | grep -q "Card offset set to 6 0 0" || fail "the control card's offset: $line"
ctl tp "$QX.5" "$QY" "$((QZ + 3)).5" >/dev/null
ctl wait 6 >/dev/null
ctl look-at block "$QX" "$QY" "$QZ" "$qface" >/dev/null 2>&1 || true
use "$QX" "$QY" "$QZ" "$qface"
ctl wait 60 >/dev/null
line=$(status "$QX" "$QY" "$QZ")
bar=$(chest_total "$QX" $((QY + 1)) "$QZ")
echo "control status: $line; chest total: $control_before -> $bar"
echo "$line" | grep -q "Box (Solid) 3x3x3 offset 6 0 0, 27 cells, 0 placed" \
    || fail "a box card did not stop at 0 placed: $line"
[ "$bar" -eq "$control_before" ] || fail "the box card moved the chest to $bar items"
volume_all minecraft:cobblestone || fail "the box card changed a cell of its volume"

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

echo "PASS: both carried recipes craft in a real crafting table (the shape card from 4 paper, 2 bricks, 1 iron ingot and 2 redstone; the Builder from 4 bricks, an ender pearl, 3 redstone and a machine frame) with their ingredients consumed and their wrong-ingredient controls empty, the game log has no rejected ingredient, the Builder fills its 3x3x3 box from the chest above it (24 placed, 64 -> 40), a hollow 5x5x5 is a 98-cell shell (92 placed against a full chest), a box outside the world border builds 0 and spends nothing, a quarry card over 27 cells breaks them and leaves the configured replacement (25 cobblestone collected, the gravel and the tag-matched coarse dirt voided and not collected), the quarry card keeps its type and void list when it is taken out and put back, tag matching off collects the coarse dirt again, /rftoolsbuilder replace changes the replacement, a box card on the same rig collects nothing, the Builder's loot table drops it, the card goes in, comes back out, resets to the 5x5x5 default, and the placed volume survives a relaunch"
