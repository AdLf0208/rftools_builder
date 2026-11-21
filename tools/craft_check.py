#!/usr/bin/env python3
"""Craft every shaped recipe this package carries, in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. The harness's
crafting table exposes the 3x3 grid as slots 100-108 (row-major) and the result
as 200 (`ctl screen`). For each recipe this:

1. resets the table and clears the inventory;
2. gives exactly the pattern's ingredients and proves they arrived;
3. places them with right-clicks (one item per slot);
4. asserts the result's **id and count**;
5. shift-clicks the result into the inventory and asserts it arrived;
6. asserts the grid is empty and a distinctive ingredient was consumed.

Each recipe has a wrong-ingredient control that must leave the result empty, so
the check cannot pass on a table that crafts whatever it is handed. Exits 1 on
the first mismatch, printing the observed grid and result.

The Builder's recipe takes `rftoolsbase:machine_frame`, so the game must carry
the `rftoolsbase` package beside this one — upstream declares rftoolsbase a
required dependency. `tools/e2e.sh` documents how that two-package game is
built; on a game without it the frame cannot be given and this says so.

    python3 tools/craft_check.py SOCK CTL X Y Z
"""
import json
import subprocess
import sys

GRID = list(range(100, 109))  # row-major: tl tm tr / ml mm mr / bl bm br
RESULT = 200


def main() -> int:
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def result():
        stack = [s for s in slots() if s["slot"] == RESULT]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def grid_is_empty():
        return all(not s.get("item") for s in slots() if s["slot"] in GRID)

    def count(item):
        return run("inventory").get("counts", {}).get(item, 0)

    def place(item, targets):
        # One pickup per target: the held stack is put back between targets, so
        # a max-stack-1 ingredient still fills only one slot at a time.
        for target in targets:
            slot = find(item)
            run("click-slot", str(slot))
            wait(1)
            run("click-slot", str(target), "right")
            wait(1)
            run("click-slot", str(slot))
            wait(1)

    def setup(ingredients):
        try:
            run("close")
        except SystemExit:
            pass
        wait(4)
        # Replace the table so its block entity, and therefore the grid, is fresh.
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, n in ingredients:
            run("give", item, str(n))
        wait(3)
        missing = [(item, n, count(item)) for item, n in ingredients if count(item) < n]
        if missing:
            raise SystemExit(
                "cannot give the recipe's ingredients "
                + ", ".join(f"{item} x{n} (have {have})" for item, n, have in missing))
        run("interact", x, y, z)
        wait(5)

    failures = []

    def check(label, ingredients, placements, want_item, want_count, consumed_item):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"  {label} -> {json.dumps(got)}")
        if not got or got.get("item") != want_item or got.get("count") != want_count:
            grid = [s for s in slots() if s["slot"] in GRID]
            print(f"    grid: {json.dumps(grid)}")
            failures.append(f"{label}: expected {want_count} {want_item}, got {got}")
            return
        run("click-slot", str(RESULT), "shift")
        wait(4)
        made = count(want_item)
        if made < want_count:
            failures.append(f"{label}: the crafted stack reached the inventory as {made} "
                            f"(the grid result was {want_count})")
        if not grid_is_empty():
            failures.append(f"{label}: the grid still holds ingredients after the craft")
        left = count(consumed_item)
        if left != 0:
            failures.append(f"{label}: {consumed_item} was not consumed ({left} left)")
        run("command", "clear @s")
        wait(2)

    def empty_control(label, ingredients, placements):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"  {label} -> {json.dumps(got)}")
        if got is not None:
            failures.append(f"{label}: the control crafted {got['item']} x{got['count']}")

    def at(*cells):
        return list(cells)

    # --- the shape card: p B p / r i r / p B p ----------------------------
    check(
        "shape card: 4 paper + 2 bricks + 1 iron ingot (c:ingots/iron) + 2 redstone",
        [
            ("minecraft:paper", 4),
            ("minecraft:bricks", 2),
            ("minecraft:iron_ingot", 1),
            ("minecraft:redstone", 2),
        ],
        [
            ("minecraft:paper", at(100, 102, 106, 108)),
            ("minecraft:bricks", at(101, 107)),
            ("minecraft:iron_ingot", at(104)),
            ("minecraft:redstone", at(103, 105)),
        ],
        "rftoolsbuilder:shape_card_def", 1, "minecraft:bricks",
    )

    # --- the Builder: B o B / r F r / B r B -------------------------------
    check(
        "builder: 4 bricks + 1 ender pearl (c:ender_pearls) + 3 redstone + machine frame",
        [
            ("minecraft:bricks", 4),
            ("minecraft:ender_pearl", 1),
            ("minecraft:redstone", 3),
            ("rftoolsbase:machine_frame", 1),
        ],
        [
            ("minecraft:bricks", at(100, 102, 106, 108)),
            ("minecraft:ender_pearl", at(101)),
            ("minecraft:redstone", at(103, 105, 107)),
            ("rftoolsbase:machine_frame", at(104)),
        ],
        "rftoolsbuilder:builder", 1, "rftoolsbase:machine_frame",
    )

    # --- controls: a wrong ingredient leaves the result empty -------------
    empty_control(
        "shape card with a stick where the paper goes",
        [
            ("minecraft:paper", 3),
            ("minecraft:stick", 1),
            ("minecraft:bricks", 2),
            ("minecraft:iron_ingot", 1),
            ("minecraft:redstone", 2),
        ],
        [
            ("minecraft:paper", at(100, 102, 106)),
            ("minecraft:stick", at(108)),
            ("minecraft:bricks", at(101, 107)),
            ("minecraft:iron_ingot", at(104)),
            ("minecraft:redstone", at(103, 105)),
        ],
    )
    empty_control(
        "builder with a stick where the ender pearl goes",
        [
            ("minecraft:bricks", 4),
            ("minecraft:stick", 1),
            ("minecraft:redstone", 3),
            ("rftoolsbase:machine_frame", 1),
        ],
        [
            ("minecraft:bricks", at(100, 102, 106, 108)),
            ("minecraft:stick", at(101)),
            ("minecraft:redstone", at(103, 105, 107)),
            ("rftoolsbase:machine_frame", at(104)),
        ],
    )

    run("close")
    wait(3)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print("recipe PASS: both carried shaped recipes craft with their declared id and count, "
          "their ingredients are consumed, the grid empties, and both wrong-ingredient "
          "controls leave the result empty")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
