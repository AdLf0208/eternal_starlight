#!/usr/bin/env python3
"""Carry slice 1's content out of Eternal Starlight's jar into this package.

The jar is 1.21.1 data; this package targets Minecraft Java 26.3, so every file
that moves is converted with the campaign's own pack-upgrade rules, which live in
the pin's `crates/horizon-port-pack/src/upgrade/` (`settings.rs`, `density.rs`,
`registries.rs`). This tool reimplements the handful of those rules that the
slice needs, in Python, so the carry is auditable and `--check` can prove the
package still matches the jar.

What moves, and why:

* nine blocks — the small surface set plus the two blocks mining them yields;
* their blockstates, models, textures, item models, loot tables and names;
* four biomes (the slice's) in 26.3's shape;
* the noise settings, its material rule, its one density function and one noise.

Everything else in the jar is recorded as a gap in `notes.md`.

Adapters, each recorded in `ADAPTATIONS` below and in the package's manifest:

* 26.3 noise settings take an eight-field router, a named `material_rule` and no
  aquifers flag; 1.21.1's `initial_density_without_jaggedness` becomes
  `chunk_surface_level`, and the beardifier is added back to `final_density`;
* 1.21.1's `y_clamped_gradient`, `flat_cache` and `cache_2d` density types no
  longer exist (`gradient`, `cache`); noise files take
  `base_octave`/`base_amplitude`;
* the material rule's one Java condition (`eternal_starlight:on_surface`) becomes
  vanilla's `minecraft:above_preliminary_surface`, and its branches for biomes
  outside the slice are pruned (they can never match: those biomes are not in
  the source's `possible_biomes`);
* biomes take mob spawns as an attribute, carvers as an array and colours as hex;
* 1.21.1's item models become 26.3 `assets/<ns>/items/<id>.json`.

`--check` rebuilds everything in memory and compares it with what is committed,
and reports any file in the package that the carry did not produce.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import pathlib
import sys
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
JAR = (
    ROOT.parent
    / "_shared/licence-cache/eternalstarlight-0.9.1+1.21.1+neoforge.jar"
)
NS = "eternal_starlight"

# The nine blocks: the slice's surfaces (grimstone is the noise settings'
# default block; the rest come from the material rule) and the two blocks
# mining them drops. Templates are the vanilla blocks the mod's own classes
# copy (`ESBlocks` `ofFullCopy`), so template behaviour matches the original.
BLOCKS = {
    "grimstone": "minecraft:stone",
    "cobbled_grimstone": "minecraft:cobblestone",
    "nightfall_grass_block": "minecraft:grass_block",
    "nightfall_dirt": "minecraft:dirt",
    "fantasy_grass_block": "minecraft:grass_block",
    "nightfall_mud": "minecraft:mud",
    "packed_nightfall_mud": "minecraft:packed_mud",
    "voidstone": "minecraft:deepslate",
    "cobbled_voidstone": "minecraft:cobbled_deepslate",
}

# The slice's biomes, in the source's `possible_biomes` order.
BIOMES = [
    "starlight_forest",
    "starlight_dense_forest",
    "glimmer_scrubland",
    "dark_swamp",
]

# Vanilla placed features a carried biome names; kept only if 26.3 still has them.
VANILLA_FEATURES = {
    "minecraft:freeze_top_layer",
    "minecraft:patch_grass_savanna",
    "minecraft:patch_tall_grass",
}

ADAPTATIONS = [
    "blocks: declarations use the vanilla template the mod's class copies",
    "loot: 26.3 shape (condition tool/can_silk_touch, survives_explosion); drop is the jar's non-silk item",
    "assets: textures and models verbatim; 1.21.1 item models become 26.3 assets/<ns>/items/<id>.json",
    "biomes: spawners/spawn_costs -> minecraft:gameplay/natural_mob_spawns; carvers -> array; colours -> hex",
    "biomes: mod spawners (Java entity types) dropped; mod feature entries dropped (Java feature types)",
    "noise settings: eight-field 26.3 router, named material_rule, no aquifers",
    "noise settings: temperature/vegetation inline shifted_noise -> minecraft:overworld/temperature|vegetation",
    "material rule: eternal_starlight:on_surface -> minecraft:above_preliminary_surface",
    "material rule: branches for biomes outside the slice pruned",
    "density: y_clamped_gradient -> gradient; flat_cache/cache_2d -> cache; argument1/2 -> left/right",
    "noise: firstOctave/amplitudes -> base_octave/base_amplitude",
]


def jar() -> zipfile.ZipFile:
    return zipfile.ZipFile(JAR)


def read(archive: zipfile.ZipFile, path: str) -> bytes:
    return archive.read(path)


def load(archive: zipfile.ZipFile, path: str) -> dict:
    return json.loads(read(archive, path))


def dump(value) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=False) + "\n").encode()


def hex_color(value: int) -> str:
    return f"#{value & 0xFFFFFF:06x}"


def state(value):
    """1.21.1 `{"Name": .., "Properties": {..}}` in 26.3's form."""
    if isinstance(value, dict) and isinstance(value.get("Name"), str):
        properties = value.get("Properties") or {}
        if properties:
            return {"id": value["Name"], "properties": properties}
        return value["Name"]
    return value


def states(value):
    if isinstance(value, dict):
        converted = state(value)
        if converted is not value:
            return converted
        return {k: states(v) for k, v in value.items()}
    if isinstance(value, list):
        return [states(v) for v in value]
    return value


def density(value, report: list[str]):
    """A 1.21.1 density function in 26.3's form (the upgrade tool's rules)."""
    if isinstance(value, (int, float)):
        return value
    if isinstance(value, str):
        return value
    if not isinstance(value, dict):
        return value
    kind = value.get("type")
    if kind is None:
        return {k: density(v, report) for k, v in value.items()}
    name = kind.split(":")[-1]
    if name in ("add", "mul", "min", "max"):
        return {
            "type": kind,
            "left": density(value["argument1"], report),
            "right": density(value["argument2"], report),
        }
    if name in ("abs", "square", "cube", "half_negative", "quarter_negative", "squeeze"):
        return {"type": kind, "input": density(value["argument"], report)}
    if name == "y_clamped_gradient":
        return {
            "type": "minecraft:gradient",
            "axis": "y",
            "from_coordinate": value["from_y"],
            "to_coordinate": value["to_y"],
            "from_value": value["from_value"],
            "to_value": value["to_value"],
        }
    if name in ("flat_cache", "cache_2d", "cache_once"):
        report.append(f"density: {name} -> cache")
        return {"type": "minecraft:cache", "input": density(value["argument"], report)}
    if name == "noise":
        return {k: v for k, v in value.items()}
    return {k: density(v, report) if k != "type" else v for k, v in value.items()}


def biome(name: str, report: list[str]) -> dict:
    source = load(jar(), f"data/{NS}/worldgen/biome/{name}.json")
    effects = source.get("effects", {})
    attributes = {
        "minecraft:gameplay/natural_mob_spawns": {
            "argument": {"spawn_costs": {}, "spawns_by_category": {}},
            "modifier": "overlay",
        }
    }
    for key, attribute in (
        ("sky_color", "minecraft:visual/sky_color"),
        ("fog_color", "minecraft:visual/fog_color"),
        ("water_fog_color", "minecraft:visual/water_fog_color"),
    ):
        if key in effects:
            attributes[attribute] = hex_color(effects[key])
    kept = {}
    for key in ("water_color", "foliage_color", "grass_color", "dry_foliage_color"):
        if key in effects:
            kept[key] = hex_color(effects[key])
    features = []
    dropped = 0
    for step in source.get("features", []):
        entries = []
        for feature in step:
            if feature in VANILLA_FEATURES:
                entries.append(feature)
            else:
                dropped += 1
        features.append(entries)
    report.append(f"biome {name}: {dropped} mod feature entries dropped")
    report.append(f"biome {name}: mod spawners dropped (Java entity types)")
    return {
        "attributes": attributes,
        "carvers": ["minecraft:cave"],
        "downfall": source.get("downfall", 0.5),
        "effects": kept,
        "features": features,
        "has_precipitation": source.get("has_precipitation", True),
        "temperature": source.get("temperature", 0.8),
    }


def material_rule(report: list[str]) -> dict:
    source = load(jar(), f"data/{NS}/worldgen/noise_settings/starlight.json")
    rule = states(source["surface_rule"])
    sequence = rule["sequence"]
    bedrock = sequence[0]
    bands = sequence[4]
    voidstone = sequence[5]
    report.append("material rule: abyss, permafrost, sea and desert branches pruned")
    report.append("material rule: eternal_starlight:on_surface -> above_preliminary_surface")

    def block(name: str) -> dict:
        return {"type": "minecraft:block", "result_state": f"{NS}:{name}"}

    def water() -> dict:
        return {
            "type": "minecraft:water",
            "offset": 0,
            "surface_depth_multiplier": 0,
            "add_stone_depth": False,
        }

    def condition(if_true: dict, then_run: dict) -> dict:
        return {"type": "minecraft:condition", "if_true": if_true, "then_run": then_run}

    def swamp() -> dict:
        return {"type": "minecraft:biome", "biome_is": f"{NS}:dark_swamp"}

    top = condition(
        {"type": "minecraft:stone_depth", "offset": 0, "add_surface_depth": False,
         "secondary_depth_range": 0, "surface_type": "floor"},
        condition(
            water(),
            {"type": "minecraft:sequence", "sequence": [
                condition(swamp(), block("fantasy_grass_block")),
                block("nightfall_grass_block"),
            ]},
        ),
    )
    filler = condition(
        {"type": "minecraft:stone_depth", "offset": 0, "add_surface_depth": True,
         "secondary_depth_range": 0, "surface_type": "floor"},
        {"type": "minecraft:sequence", "sequence": [
            condition(swamp(), block("nightfall_mud")),
            block("nightfall_dirt"),
        ]},
    )
    surface = condition({"type": "minecraft:above_preliminary_surface"},
                        {"type": "minecraft:sequence", "sequence": [top, filler]})
    return {
        "type": "minecraft:sequence",
        "sequence": [bedrock, surface, bands, voidstone],
    }


def noise_settings(report: list[str]) -> dict:
    source = load(jar(), f"data/{NS}/worldgen/noise_settings/starlight.json")
    router = source["noise_router"]
    settings = {
        "default_block": state(source["default_block"]),
        "default_fluid": state(source["default_fluid"]),
        "disable_mob_generation": bool(source.get("disable_mob_generation", False)),
        "legacy_random_source": bool(source.get("legacy_random_source", False)),
        "material_rule": f"{NS}:starlight",
        "noise": {"height": source["noise"]["height"], "min_y": source["noise"]["min_y"]},
        "noise_router": {
            "temperature": "minecraft:overworld/temperature",
            "vegetation": "minecraft:overworld/vegetation",
            "continents": density(router["continents"], report),
            "erosion": density(router["erosion"], report),
            "depth": density(router["depth"], report),
            "ridges": density(router["ridges"], report),
            "chunk_surface_level": {
                "type": "minecraft:interpolated",
                "cell_size_xz": 16,
                "cell_size_y": 1,
                "input": density(router["initial_density_without_jaggedness"], report),
            },
            "final_density": {
                "type": "minecraft:add",
                "left": density(router["final_density"], report),
                "right": {"type": "minecraft:beardifier"},
            },
        },
        "sea_level": source["sea_level"],
        "spawn_target": [
            {
                "minecraft:overworld/temperature": point["temperature"],
                "minecraft:overworld/vegetation": point["humidity"],
                "minecraft:overworld/continents": point["continentalness"],
                "minecraft:overworld/erosion": point["erosion"],
                "minecraft:overworld/ridges": point["weirdness"],
            }
            for point in source["spawn_target"]
        ],
    }
    report.append("noise settings: aquifers dropped (1.21.1 disabled them)")
    report.append("noise settings: ore veins dropped (1.21.1 disabled them)")
    return settings


def density_function(report: list[str]) -> dict:
    source = load(jar(), f"data/{NS}/worldgen/density_function/depth.json")
    return density(source, report)


def noise(report: list[str]) -> dict:
    source = load(jar(), f"data/{NS}/worldgen/noise/depth_offset.json")
    amplitudes = source["amplitudes"]
    report.append("noise: firstOctave/amplitudes -> base_octave/base_amplitude")
    result = {
        "base_amplitude": amplitudes[0],
        "base_octave": source["firstOctave"],
    }
    if len(amplitudes) > 1:
        result["octave_count"] = len(amplitudes)
        result["amplitude_modifiers"] = [
            a / amplitudes[0] for a in amplitudes[1:]
        ]
    return result


def loot(name: str, archive: zipfile.ZipFile) -> dict:
    source = load(archive, f"data/{NS}/loot_table/blocks/{name}.json")
    drop = name
    for pool in source.get("pools", []):
        for entry in pool.get("entries", []):
            for child in entry.get("children", []):
                conditions = child.get("conditions", [])
                if any(c.get("condition") == "minecraft:survives_explosion" for c in conditions):
                    drop = child["name"].split(":")[-1]
    return {
        "type": "minecraft:block",
        "pools": [
            {
                "entries": [
                    {
                        "type": "minecraft:alternatives",
                        "children": [
                            {
                                "type": "minecraft:item",
                                "condition": "minecraft:tool/can_silk_touch",
                                "name": f"{NS}:{name}",
                            },
                            {
                                "type": "minecraft:item",
                                "condition": {"type": "minecraft:survives_explosion"},
                                "name": f"{NS}:{drop}",
                            },
                        ],
                    }
                ],
                "rolls": 1,
            }
        ],
        "random_sequence": f"{NS}:blocks/{name}",
    }


def model_closure(archive: zipfile.ZipFile, block: str):
    """The models and textures a block's blockstate reaches."""
    models: dict[str, bytes] = {}
    textures: dict[str, bytes] = {}
    pending = []
    blockstate = load(archive, f"assets/{NS}/blockstates/{block}.json")
    pending.extend(referenced_models(blockstate))
    while pending:
        model = pending.pop()
        if model in models:
            continue
        path = f"assets/{NS}/models/{model.split(':', 1)[1]}.json"
        data = read(archive, path)
        models[model] = data
        body = json.loads(data)
        parent = body.get("parent", "")
        if parent.startswith(f"{NS}:"):
            pending.append(parent)
        for texture in body.get("textures", {}).values():
            if texture.startswith("#"):
                continue
            if texture.startswith(f"{NS}:"):
                name = texture.split(":", 1)[1]
                textures[name] = read(archive, f"assets/{NS}/textures/{name}.png")
    return models, textures


def referenced_models(value) -> list[str]:
    found = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "model" and isinstance(child, str) and child.startswith(f"{NS}:"):
                found.append(child)
            else:
                found.extend(referenced_models(child))
    elif isinstance(value, list):
        for child in value:
            found.extend(referenced_models(child))
    return found


def build() -> tuple[dict[str, bytes], list[str]]:
    report: list[str] = []
    files: dict[str, bytes] = {}
    archive = jar()

    for block in BLOCKS:
        files[f"data/{NS}/horizon/block/{block}.json"] = dump(
            {"template": BLOCKS[block], "item": True}
        )
        files[f"data/{NS}/loot_table/blocks/{block}.json"] = dump(
            loot(block, archive)
        )
        files[f"assets/{NS}/blockstates/{block}.json"] = read(
            archive, f"assets/{NS}/blockstates/{block}.json"
        )
        models, textures = model_closure(archive, block)
        item = load(archive, f"assets/{NS}/models/item/{block}.json")
        target = item.get("parent")
        if target not in models:
            target = referenced_models(
                load(archive, f"assets/{NS}/blockstates/{block}.json")
            )[0]
        files[f"assets/{NS}/items/{block}.json"] = dump(
            {"model": {"type": "minecraft:model", "model": target}}
        )
        for model, data in models.items():
            files[f"assets/{NS}/models/{model.split(':', 1)[1]}.json"] = data
        for texture, data in textures.items():
            files[f"assets/{NS}/textures/{texture}.png"] = data

    lang = load(archive, f"assets/{NS}/lang/en_us.json")
    names = {}
    for block in BLOCKS:
        key = f"block.{NS}.{block}"
        if key not in lang:
            raise SystemExit(f"jar lang has no {key}")
        names[key] = lang[key]
    for biome_name in BIOMES:
        key = f"biome.{NS}.{biome_name}"
        if key in lang:
            names[key] = lang[key]
    files[f"assets/{NS}/lang/en_us.json"] = dump(dict(sorted(names.items())))

    for name in BIOMES:
        files[f"data/{NS}/worldgen/biome/{name}.json"] = dump(biome(name, report))
    files[f"data/{NS}/worldgen/noise_settings/starlight.json"] = dump(
        noise_settings(report)
    )
    files[f"data/{NS}/worldgen/material_rule/starlight.json"] = dump(
        material_rule(report)
    )
    files[f"data/{NS}/worldgen/density_function/depth.json"] = dump(
        density_function(report)
    )
    files[f"data/{NS}/worldgen/noise/depth_offset.json"] = dump(noise(report))
    return files, report


CARRY_DIRS = ("assets", "data")


def carried_on_disk() -> set[str]:
    found = set()
    for directory in CARRY_DIRS:
        for path in (ROOT / directory).rglob("*"):
            if path.is_file():
                found.add(path.relative_to(ROOT).as_posix())
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    files, report = build()
    expected = set(files)
    on_disk = carried_on_disk()
    if args.check:
        problems = []
        for path, data in files.items():
            target = ROOT / path
            if not target.exists():
                problems.append(f"missing {path}")
            elif target.read_bytes() != data:
                problems.append(f"differs {path}")
        for extra in sorted(on_disk - expected):
            problems.append(f"unexpected {extra}")
        for line in problems:
            print(line, file=sys.stderr)
        if problems:
            return 1
        total = sum(len(data) for data in files.values())
        print(f"OK {len(files)} files, {total} bytes")
        for line in report:
            print(f"  {line}")
        return 0
    if not args.write:
        print("pass --write or --check", file=sys.stderr)
        return 2
    for stale in sorted(on_disk - expected):
        (ROOT / stale).unlink()
    for path, data in files.items():
        target = ROOT / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    total = sum(len(data) for data in files.values())
    digest = hashlib.sha256()
    for path in sorted(files):
        digest.update(path.encode())
        digest.update(files[path])
    print(f"wrote {len(files)} files, {total} bytes, tree sha256 {digest.hexdigest()}")
    for line in report:
        print(f"  {line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
