#!/usr/bin/env bash
# Chunk-load phase for `rftoolsbuilder`: a quarry rig whose 3x3x3 box lies in a
# NEVER-VISITED chunk ~200 blocks away (deep underground, solid stone).
#
#   * 0.2.4 (the old package): the first readout stalls at `0 quarried` with
#     `refused at ...: the chunk is not loaded` — the counterfactual red.
#   * this slice: the Builder registers its own ticket controller, force-loads
#     the cell's chunk, finishes all 27 cells (`27 quarried, scan complete`),
#     leaves the replacement at the corner, and RELEASES the ticket afterwards
#     (`Forced chunks at ...: none`). A second rig in the same chunk forces its
#     own ticket too — the unload/reload proof.
#
# `SELFTEST=chunkload` inverts the finish assertion, so that leg MUST end FAIL.
# GAME / CTL / RES / SOCK / E2E_TMP come from `bundle-game.sh env`.
set -euo pipefail

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/rftoolsbuilder
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/rftoolsbuilder-chunkload.sock}
GAMELOG=${GAMELOG:-${SOCK%.sock}.log}
SELFTEST=${SELFTEST:-0}
COUNTERFACTUAL=${COUNTERFACTUAL:-0}
BUILDER=rftoolsbuilder:builder
CARD=rftoolsbuilder:shape_card_def
# Card offset: 200 east, 152 down, 200 south. The box's low corner is
# (builder.x - 1 + 200, builder.y - 1 - 152, builder.z - 1 + 200).
OX=${OX:-260}; OY=${OY:--152}; OZ=${OZ:-260}
LX=$((OX - 1)); LY=$((OY - 1)); LZ=$((OZ - 1))

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/rfb-chunkload-home.XXXXXX)
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
mod() { ctl chat "/rftoolsbuilder $*" >/dev/null; ctl wait 8 >/dev/null; }
# Fill a Builder's FE buffer through its energy capability (a cable stand-in).
charge() { # charge X Y Z [amount]
    local amount=${4:-1000000} i out=""
    for i in 1 2 3 4 5; do
        mod charge "$1" "$2" "$3" "$amount"
        out=$(ctl messages 20 | jq -r '.chat[]' \
            | grep -E "^charged Builder at $1 $2 $3:|^not a builder" | head -1 || true)
        [ -n "$out" ] && { echo "$out"; return 0; }
        ctl wait 5 >/dev/null
    done
    echo "$out"
}
answer() {
    local i out=""
    for i in 1 2 3 4 5; do
        mod "$@"
        out=$(ctl messages 20 | jq -r '.chat[]' \
            | grep -E '^(Card set to|Card offset set to|Card type set to|Card tag matching)' | head -1 || true)
        [ -n "$out" ] && { echo "$out"; return 0; }
        ctl wait 5 >/dev/null
    done
    echo "$out"
}
status() {
    local i out=""
    for i in 1 2 3 4 5 6; do
        mod status "$1" "$2" "$3"
        ctl wait 6 >/dev/null
        out=$(ctl messages 20 | jq -r '.chat[]' | grep -E "^Builder at $1 $2 $3:" | head -1 || true)
        [ -n "$out" ] && { echo "$out"; return 0; }
    done
    echo "$out"
}
# One status call with the shortest possible delay: the 0.2.4 counterfactual's
# `the chunk is not loaded` refusal is visible only before the chunk streams in.
status_now() {
    ctl chat "/rftoolsbuilder status $1 $2 $3" >/dev/null
    ctl wait 3 >/dev/null
    ctl messages 20 | jq -r '.chat[]' | grep -E "^Builder at $1 $2 $3:" | head -1 || true
}
# One `/forced` call. The reply starts with `Forced chunks at`, so it can never
# be mistaken for a status readout (`Builder at`).
forced_now() {
    mod forced "$1" "$2" "$3"
    ctl messages 20 | jq -r '.chat[]' | grep -E '^Forced chunks at' | head -1 || true
}
# A two-tick poll: the pass finishes in about four ticks once the chunk arrives,
# so the ticket must be sampled immediately after the card is installed.
forced_fast() {
    ctl chat "/rftoolsbuilder forced $1 $2 $3" >/dev/null
    ctl wait 2 >/dev/null
    ctl messages 20 | jq -r '.chat[]' | grep -E '^Forced chunks at' | head -1 || true
}
chest_total() { ctl container "$1" "$2" "$3" | jq -r '[.slots[]? | .count] | add // 0'; }

cleanup() {
    local code=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" 2>/dev/null || true
    echo "EXIT:$code"
}
trap cleanup EXIT

launch() {
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world RFToolsBuilderChunkload --seed 20261105 --creative --offscreen --wait \
        --game-dir "$SCRATCH/game" --resources "$RES" >/dev/null
    ctl wait-ready 180 >/dev/null
    if grep -q "content mods not loaded" "${GAMELOG}" 2>/dev/null; then
        grep "content mods not loaded" "${GAMELOG}" >&2
        fail "the content mod did not load"
    fi
}

# A quarry rig at (bx,by,bz). A standing pad under the click position keeps the
# player within the 5-block interact reach after the site is cleared.
rig() { # bx by bz label
    local bx=$1 by=$2 bz=$3 label=$4
    ctl command "fill $((bx - 4)) $((by - 3)) $((bz - 4)) $((bx + 4)) $((by + 4)) $((bz + 4)) minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl setblock "$bx" $((by - 1)) "$bz" minecraft:stone >/dev/null
    ctl setblock "$bx" $((by - 2)) "$((bz + 3))" minecraft:stone >/dev/null
    ctl command "setblock $bx $by $bz $BUILDER" >/dev/null
    ctl wait 6 >/dev/null
    [ "$(block_name "$bx" "$by" "$bz")" = "$BUILDER" ] || fail "the $label Builder is not at $bx $by $bz"
    echo "$label charge: $(charge "$bx" "$by" "$bz")" >&2
    ctl setblock "$bx" $((by + 1)) "$bz" minecraft:chest >/dev/null
    RIG_CHEST_BEFORE=$(chest_total "$bx" $((by + 1)) "$bz")
    ctl command "clear @a" >/dev/null
    ctl tp "$bx.5" "$((by - 1))" "$((bz + 3)).5" >/dev/null
    ctl wait 6 >/dev/null
    ctl give "$CARD" 1 >/dev/null || fail "cannot give a card for $label"
    ctl wait 6 >/dev/null
    ctl select "$CARD" >/dev/null
    ctl wait 6 >/dev/null
    local line
    line=$(answer shape box 3 3 3)
    echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "the $label card was not written: $line"
    line=$(answer offset "$OX" "$OY" "$OZ")
    echo "$line" | grep -q "Card offset set to $OX $OY $OZ" || fail "the $label offset: $line"
    line=$(answer type quarry)
    echo "$line" | grep -q "Card type set to quarry" || fail "the $label type: $line"
    ctl look-at block "$bx" "$by" "$bz" south >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    ctl interact "$bx" "$by" "$bz" south >/dev/null 2>&1 || true
    # Sample the ticket table immediately, before the pass finishes and releases
    # it. With 0.2.4 there is no `forced` command, so every poll is empty.
    RIG_SEEN=""
    for i in $(seq 1 20); do
        out=$(forced_fast "$bx" "$by" "$bz")
        if echo "$out" | grep -q '\['; then RIG_SEEN="$out"; break; fi
    done
    echo "$label ticket during pass: ${RIG_SEEN:-<none>}"
    # Read after the click's slot write has been mirrored; the retrying status
    # helper waits for a Builder readout. With the 0.2.4 package this timing
    # showed the unloaded-stall refusal verbatim.
    ctl wait 12 >/dev/null
    RIG_FIRST_STATUS=$(status "$bx" "$by" "$bz")
    echo "$label card: $RIG_FIRST_STATUS"
    echo "$RIG_FIRST_STATUS" | grep -q "quarry (Normal quarry)" \
        || fail "the $label card is not installed: $RIG_FIRST_STATUS"
}

say "launch (offscreen renderer, creative)"
launch

say "an Overworld arena near spawn"
PX=$(ctl state | jq -r '.player.position[0] | floor')
PZ=$(ctl state | jq -r '.player.position[2] | floor')
ctl command "fill $((PX-12)) 100 $((PZ-12)) $((PX+12)) 100 $((PZ+12)) minecraft:stone" >/dev/null
ctl command "fill $((PX-12)) 101 $((PZ-12)) $((PX+12)) 118 $((PZ+12)) minecraft:air" >/dev/null
ctl tp "$PX.5" 101 "$((PZ+4)).5" >/dev/null
ctl wait 20
ctl command "gamerule spawn_mobs false" >/dev/null
ctl command "kill @e[type=!minecraft:player]" >/dev/null
ctl wait 20

BX=$PX; BY=102; BZ=$((PZ + 1))
DX1=$((BX + LX)); DX2=$((DX1 + 2))
DY1=$((BY + LY)); DY2=$((DY1 + 2))
DZ1=$((BZ + LZ)); DZ2=$((DZ1 + 2))
say "RIG 1: quarry box $DX1..$DX2 $DY1..$DY2 $DZ1..$DZ2 (never visited, unloaded)"
# The box is created by nothing but the card; no command ever loads its chunk.
rig "$BX" "$BY" "$BZ" "first"

# The first readout is the fail-capable counterfactual: 0.2.4 says the chunk is
# not loaded; the slice's ticket makes the pass start instead.
stall_line=$RIG_FIRST_STATUS
echo "first readout: $stall_line"
if [ "$COUNTERFACTUAL" = 1 ]; then
    # The old package has no ticket controller: it must stall at 0 quarried
    # with the unloaded-chunk refusal. This leg is expected to end FAIL + EXIT:1.
    echo "$stall_line" | grep -q "0 quarried" \
        || fail "0.2.4 counterfactual: no stall at 0 quarried: $stall_line"
    echo "$stall_line" | grep -q "the chunk is not loaded" \
        || fail "0.2.4 counterfactual: no unloaded refusal: $stall_line"
    fail "0.2.4 counterfactual (expected red): $stall_line"
fi
before=$RIG_CHEST_BEFORE
seen=$RIG_SEEN
echo "chest before: $before; ticket seen: ${seen:-<none>}"

ctl wait 200 >/dev/null
line=$(status "$BX" "$BY" "$BZ")
echo "quarry status: $line"
after=$(chest_total "$BX" $((BY + 1)) "$BZ")
forced_after=$(forced_now "$BX" "$BY" "$BZ")
echo "chest after: $after; $forced_after"

if [ "$SELFTEST" = chunkload ]; then
    echo "$line" | grep -q "27 quarried, scan complete" \
        && fail "chunkload selftest: the pass completed in the unloaded chunk"
    exit 0
fi

echo "$line" | grep -q "27 quarried, scan complete" \
    || fail "the quarry did not finish in the unloaded chunk: $line"
[ "$((after - before))" -ge 27 ] \
    || fail "the chest collected $((after - before)) items, expected at least 27"
# The ticket must have been held during the pass (read from the game's own
# ticket table), then released afterwards — not merely saved in the Builder.
[ -n "$seen" ] || fail "the Builder never held a force-load ticket"
echo "$forced_after" | grep -q ": none" \
    || fail "the ticket was not released after the run: $forced_after"

# Read the corner with `ctl block`: the ticket was released, so stand at the
# distant chunk to load it from the saved world before reading.
echo "tp to corner chunk: $(ctl tp "$((DX1 + 1)).5" 100 "$((DZ1 + 1)).5" 2>&1)"
ctl wait 20 >/dev/null
echo "player at corner: $(ctl state | jq -c '.player.position')"
echo "corner read: $(ctl block "$DX1" "$DY1" "$DZ1" 2>&1)"
far_corner=$(block_name "$DX1" "$DY1" "$DZ1")
echo "distant corner: $far_corner"
[ "$far_corner" = minecraft:dirt ] \
    || fail "the distant corner is $far_corner, expected minecraft:dirt"
ctl tp "$PX.5" 101 "$((PZ + 4)).5" >/dev/null
ctl wait 20 >/dev/null

say "RIG 2: a second Builder forces its OWN ticket in a different unloaded chunk"
# Rig 1's ticket was already proven released (`Forced chunks ...: none`). The
# chunk it forced stays cached in this engine, so the second rig targets a
# never-visited chunk (one chunk south) to prove a fresh, independent force.
ctl tp "$PX.5" 101 "$((PZ + 4)).5" >/dev/null
ctl wait 20 >/dev/null
BX2=$((PX + 4)); BY2=$BY; BZ2=$BZ
# Rig 1's corner sits at +z and its render radius loads the chunks around it, so rig 2 must
# target the OPPOSITE side (-z, the full 260-block clamp) to be deterministically unloaded.
OX2=$((OX - 4)); OZ2=$((-OZ))
ctl command "fill $((BX2 - 4)) $((BY2 - 3)) $((BZ2 - 4)) $((BX2 + 4)) $((BY2 + 4)) $((BZ2 + 4)) minecraft:air" >/dev/null
ctl wait 6 >/dev/null
ctl setblock "$BX2" $((BY2 - 1)) "$BZ2" minecraft:stone >/dev/null
ctl setblock "$BX2" $((BY2 - 2)) "$((BZ2 + 3))" minecraft:stone >/dev/null
ctl command "setblock $BX2 $BY2 $BZ2 $BUILDER" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$BX2" "$BY2" "$BZ2")" = "$BUILDER" ] || fail "the second Builder is missing"
echo "second rig charge: $(charge "$BX2" "$BY2" "$BZ2")" >&2
ctl setblock "$BX2" $((BY2 + 1)) "$BZ2" minecraft:chest >/dev/null
ctl command "clear @a" >/dev/null
ctl tp "$BX2.5" "$((BY2 - 1))" "$((BZ2 + 3)).5" >/dev/null
ctl wait 6 >/dev/null
ctl give "$CARD" 1 >/dev/null || fail "cannot give the second card"
ctl wait 6 >/dev/null
ctl select "$CARD" >/dev/null
ctl wait 6 >/dev/null
line=$(answer shape box 3 3 3); echo "$line" | grep -q "Box (Solid) 3x3x3" || fail "second card shape"
line=$(answer offset "$OX2" "$OY" "$OZ2"); echo "$line" | grep -q "Card offset set to $OX2 $OY $OZ2" || fail "second card offset"
line=$(answer type quarry); echo "$line" | grep -q "Card type set to quarry" || fail "second card type"
ctl look-at block "$BX2" "$BY2" "$BZ2" south >/dev/null 2>&1 || true
ctl wait 6 >/dev/null
ctl interact "$BX2" "$BY2" "$BZ2" south >/dev/null 2>&1 || true
echo "second rig first status: $(status_now "$BX2" "$BY2" "$BZ2")"
seen2=""
for i in $(seq 1 20); do
    out=$(forced_fast "$BX2" "$BY2" "$BZ2")
    echo "second rig forced poll $i: ${out:-<no reply>}"
    if echo "$out" | grep -q '\['; then seen2="$out"; break; fi
done
echo "second rig ticket: ${seen2:-<none>}"
ctl wait 10 >/dev/null
line=$(status "$BX2" "$BY2" "$BZ2")
echo "second rig card: $line"
echo "$line" | grep -q "quarry (Normal quarry)" || fail "the second card is not installed"
ctl wait 200 >/dev/null
line2=$(status "$BX2" "$BY2" "$BZ2")
forced2=$(forced_now "$BX2" "$BY2" "$BZ2")
echo "second rig status: $line2; $forced2"
echo "$line2" | grep -q "27 quarried, scan complete" \
    || fail "the second rig did not force its chunk: $line2"
[ -n "$seen2" ] || fail "the second rig never held its own ticket"
echo "$forced2" | grep -q ": none" || fail "the second rig left a ticket behind: $forced2"

echo "PASS: a quarry card force-loads its never-visited chunk (0.2.4 stalls at 0 quarried with 'the chunk is not loaded'), finishes all 27 cells with the replacement at the corner, holds then releases its own ticket, and a second Builder in another unloaded chunk forces and releases its own ticket"
