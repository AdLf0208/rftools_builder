#!/usr/bin/env python3
"""Check every carried recipe file against the 26.3 recipe dialect.

Horizon 26.3's recipe loader reads an ingredient as a **string**: a bare item id
(`minecraft:bricks`) or a tag with a leading `#` (`#c:ender_pearls`). The 1.21.1
NeoForge shape (`{"item": "minecraft:bricks"}` / `{"tag": "c:ender_pearls"}`)
is refused, and the whole recipe is dropped with

    recipes: invalid ingredient (unresolved tag or malformed input): {...}

so a carried recipe in the old shape is dead data that `check`, `fits` and the
unit tests all pass. This guard is the cheap, repeatable half of the proof; the
in-game craft in `tools/e2e.sh` is the other half.

    python3 tools/check_recipes.py [PORT]     # exit 1 on any old-shape ingredient
"""
import json
import pathlib
import sys


def ingredients(recipe):
    """Every ingredient value a recipe file declares, with a path to it."""
    kind = recipe.get("type", "minecraft:crafting_shaped")
    found = []
    if kind == "minecraft:crafting_shaped":
        for key, value in sorted(recipe.get("key", {}).items()):
            found.append((f"key.{key}", value))
    elif kind == "minecraft:crafting_shapeless":
        for i, value in enumerate(recipe.get("ingredients", [])):
            found.append((f"ingredients[{i}]", value))
    elif kind in ("minecraft:smelting", "minecraft:blasting", "minecraft:smoking",
                  "minecraft:campfire_cooking", "minecraft:stonecutting"):
        found.append(("ingredient", recipe.get("ingredient")))
    return found


def wrong_shape(value):
    """Why a value is not a 26.3 ingredient, or None when it is one."""
    if isinstance(value, str):
        if not value:
            return "empty string"
        return None
    if isinstance(value, list):
        bad = [v for v in value if not isinstance(v, str)]
        if bad:
            return f"a list holding a non-string ({bad[0]!r})"
        return None
    if isinstance(value, dict):
        keys = ", ".join(sorted(value))
        return f"an object with {keys}"
    return f"{type(value).__name__}"


def main() -> int:
    port = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else
                        pathlib.Path(__file__).resolve().parent.parent)
    files = sorted(port.glob("data/*/recipe/**/*.json"))
    if not files:
        print(f"FAIL: no recipe files under {port}/data", file=sys.stderr)
        return 1
    failures = []
    for path in files:
        try:
            recipe = json.loads(path.read_text())
        except json.JSONDecodeError as error:
            failures.append(f"{path.relative_to(port)}: invalid JSON ({error})")
            continue
        for where, value in ingredients(recipe):
            problem = wrong_shape(value)
            if problem is not None:
                failures.append(
                    f"{path.relative_to(port)}: {where} is {problem}: {json.dumps(value)}")
        result = recipe.get("result")
        if isinstance(result, dict):
            if not isinstance(result.get("id"), str):
                failures.append(f"{path.relative_to(port)}: result has no string id")
            count = result.get("count", 1)
            if not isinstance(count, int) or isinstance(count, bool) or not 1 <= count <= 99:
                failures.append(f"{path.relative_to(port)}: result count {count!r} is not 1-99")
        elif result is not None and not isinstance(result, str):
            failures.append(f"{path.relative_to(port)}: result is {type(result).__name__}")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        print(f"recipes FAIL: {len(failures)} old-shape or malformed ingredient(s) in "
              f"{len(files)} file(s)", file=sys.stderr)
        return 1
    print(f"recipes PASS: {len(files)} carried recipe files use 26.3 bare item/tag ingredients")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
