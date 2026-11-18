# Eternal Starlight — port notes (slice 1)

Ported from **Eternal Starlight `0.9.1+1.21.1+neoforge`** (NeoForge, Minecraft
1.21.1) for Minecraft Java **26.3** on the SDK pin
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, `api = "0.2"`, launcher 0.2.78).
One flat `.hmod`: `horizon-mod.toml` + `Cargo.toml` + `src/` + `assets/` +
`data/`, built by the launcher against the pinned game.

**Class + tier:** `content` / `deep` (`ports/_shared/source-map/atm10-sources.csv`
line 164: `eternal_starlight,Eternal Starlight,content,deep,MIT,…`; ledger row
`eternal_starlight,…,pending,P0`). The approach is the tier's **agent port** from
`horizon/docs/modpacks.md` — a dimension is "JSON part with worldgen; custom
generators as mod code" (modpacks.md §"What a content pack needs", the
Dimensions row) — with the pure logic in `src/runtime.rs` and the registrations
in one `impl Hooks`. The carry-over half is the block/biome/settings JSON.

**Licence: MIT → rank 0.** Read from the jar's own
`META-INF/neoforge.mods.toml` line 4 (`license = "MIT"`; line 15
`authors = "LeoLezury"`), matching `atm10-sources.csv`. The upstream
repository's `LICENSE` is the MIT text (`Copyright (c) 2024
LeoMinecraftModding`), reproduced as this port's `LICENSE`; `LICENSE-NOTICE.md`
records the provenance. **Local-only until the maintainer approves publishing**
(`ARCHITECTURE.md` §8) — MIT makes publication possible, it does not grant it.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Provenance

`eternalstarlight-0.9.1+1.21.1+neoforge.jar`, **59,325,382 B**, SHA-256
`e77369ad1a779036b243ef0f7b24776640a3ff84968e6f3be1931bc953e92b09`, already in
`ports/_shared/licence-cache/` (the pack's own file; ATM10 ships this version).

Upstream analysis (authorised, and an aid only — the port is a clean-room
rewrite, no Java was translated): `https://github.com/LeoMinecraftModding/eternal-starlight`,
shallow clone at commit **`1053e82b32cc839c16e26f0b74b2208c3473cc42`**
(2026-11-17). The clone is deleted after this record; what was read:

- `common/src/main/java/…/registry/ESBiomeSourceTypes.java`, `ESChunkGeneratorTypes.java`
  — the two Java types the jar's `dimension/starlight.json` names
  (`eternal_starlight:multi_noise`, `eternal_starlight:biome_based`);
- `…/world/gen/biomesource/ESBiomeSource.java` — the source is vanilla's
  `Climate.ParameterList.findValue` over `surfaceClimate` (the jar's
  `biome_data_parameters`), with river lookup by density function;
- `…/world/gen/chunkgenerator/ESChunkGenerator.java` — **extends
  `NoiseBasedChunkGenerator`**, adding only river fluid filling (this is
  `main`; the 0.9.1 class differs — see the generator note below);
- `…/registry/ESBlocks.java` lines 339–380, 520–522, 764–770 — every carried
  block's `BlockBehaviour.Properties` is `ofFullCopy` of a vanilla block, which
  is where the templates come from;
- `…/data/ESConfiguredFeatures.java` — the mod's features are Java feature
  configurations; nothing to carry.

Behaviours extracted: the biome table is a nearest-point search over 11,369
parameter points; a biome's `biome_data` carries `height`, `variance`, `fluid`,
`is_ocean`, `has_rivers`; the surface rule is the jar's `surface_rule`; blocks
copy vanilla classes and numbers.

## Jar inventory (measured, not guessed)

`unzip -l` over the 59 MB jar: **7,986 JSON**, 1,451 PNG, 77 OGG, 36 `.mcmeta`,
24 `.nbt`, 8 `.fsh`, 3 `.vsh`.

| Family | Count | Note |
|---|---:|---|
| `assets/eternal_starlight/blockstates` | 790 | 791 entries counting the folder (the brief's number) |
| `assets/eternal_starlight/models/block` | 1,526 | |
| `assets/eternal_starlight/models/item` | 1,196 | 1.21.1 shape |
| `assets/eternal_starlight/textures/block` | 617 | |
| `assets/eternal_starlight/textures/item` | 433 | |
| `assets/eternal_starlight/lang` | 5 | `en_us` + 4 translations |
| `data/eternal_starlight/recipe` | 1,283 | |
| `data/eternal_starlight/loot_table` | 832 | 784 of them `blocks/` |
| `data/**/tags` | 440 | |
| `data/eternal_starlight/worldgen/biome` | 23 | the brief said 34; measured 23 biome files (+25 `biome_data`) |
| `data/eternal_starlight/worldgen/noise_settings` | 1 | `starlight.json`, 19,694 B |
| `data/eternal_starlight/worldgen/configured_feature` | 118 | |
| `data/eternal_starlight/worldgen/placed_feature` | 114 | |
| `data/eternal_starlight/worldgen/structure` | 8 | + 24 `.nbt` templates |
| `data/eternal_starlight/worldgen/structure_set` | 8 | |
| `data/eternal_starlight/worldgen/template_pool` | 9 | |
| `data/eternal_starlight/worldgen/density_function` | 1 | `depth.json` |
| `data/eternal_starlight/worldgen/noise` | 1 | `depth_offset.json` |
| `data/eternal_starlight/worldgen/configured_carver` | 2 | (`caves`, and one more) |
| `data/eternal_starlight/worldgen/processor_list` | 2 | |
| `data/eternal_starlight/eternal_starlight/biome_data` | 25 | the mod's own registry |
| `data/eternal_starlight/dimension/starlight.json` | 1 | 6,994,553 B: 11,369 parameter points |
| `data/eternal_starlight/dimension_type/starlight.json` | 1 | 580 B |

A full dimension mod, so the slice below is one bounded part of it, not the mod.

## The slice (and why exactly this)

Ship: **the dimension exists, is enterable and generates** for four biomes, with
the blocks those surfaces use and a readout command.

1. **Dimension.** `eternal_starlight:starlight`, registered through
   `Registrar::register_dimension` from the fields of the jar's
   `dimension_type/starlight.json`: `min_y -64`, `height 384`,
   `logical_height 384`, no ceiling, skylight, `coordinate_scale 1.0`,
   `ambient_light 0.0`, monster spawn light 0..=7 and block limit 7,
   `respawn_anchor_works`, no raids, no piglin, sea level 48, fixed time. The
   dimension is declared in Rust rather than carried as data because the
   biome source must be the mod's own (`mod_biome_source`), and the client
   effect id (`eternal_starlight:special_effect`) has no `DimensionSettings`
   field.
2. **Biome source.** `eternal_starlight:multi_noise` is a Java type, so
   `StarlightBiomes` implements the SDK's `BiomeSource`. Its rows are the jar's
   `biome_data_parameters` rows for the four biomes nearest the origin, copied
   exactly (`src/table.rs`, 4,884 rows, generated by `tools/gen_table.py`):
   starlight forest (1,350 rows), dense forest (486), glimmer scrubland (1,242),
   dark swamp (1,806). `nearest` is vanilla's `Climate.Parameter.distance` sum
   in the jar's row order, so a tie resolves as the jar's would.
3. **Surface and blocks.** The noise settings, the material rule (the jar's
   surface rule), one density function and one noise are carried and converted;
   the nine blocks are `grimstone` (the settings' `default_block`), the rule's
   seven surface blocks, and the two blocks mining them drops
   (`cobbled_grimstone`, `cobbled_voidstone`), with blockstates, models,
   textures, item models, loot tables and names.
4. **Structures: none.** Every one of the jar's eight fails item 4's test — see
   the gaps, with file and line.
5. **Readout.** `/eternal_starlight status [<x> <y> <z>]` answers with the
   dimension id, the biome count and the biome at the position's quart cell,
   plus the dry surface block the rule places for it.
6. **Pure logic.** `src/runtime.rs` holds the distance, the climate, the grid,
   the column height, the surface/band layers and the readout, with 19 tests.

## File map (jar → package)

| Jar | Package | Change |
|---|---|---|
| `dimension_type/starlight.json` | `src/lib.rs` `settings()` | declared in Rust (fields above) |
| `dimension/starlight.json` `biome_data_parameters` | `src/table.rs` | 4,884 rows, exact endpoints |
| `worldgen/biome/{4}.json` | `data/eternal_starlight/worldgen/biome/` | 26.3 shape (see adaptations) |
| `worldgen/noise_settings/starlight.json` | `.../worldgen/noise_settings/starlight.json` | 26.3 eight-field router, named material rule |
| `worldgen/noise_settings` `surface_rule` | `.../worldgen/material_rule/starlight.json` | 26.3 registry, one Java condition replaced, pruned |
| `worldgen/density_function/depth.json` | same path | 26.3 density types |
| `worldgen/noise/depth_offset.json` | same path | 26.3 noise shape |
| `blockstates`, `models/block`, `textures/block` (9 blocks' closure) | `assets/eternal_starlight/…` | verbatim |
| `models/item/{9}.json` | `assets/eternal_starlight/items/{9}.json` | 26.3 item model |
| `loot_table/blocks/{9}.json` | same path | 26.3 shape |
| `lang/en_us.json` (13 keys) | `assets/eternal_starlight/lang/en_us.json` | subset |
| the nine block classes (`ESBlocks`) | `data/eternal_starlight/horizon/block/*.json` | template = the copied vanilla block |

`tools/carry_content.py --check` proves 70 files / 30,297 B and that no carried
file differs from what the jar plus the recorded adaptations produce;
`tools/gen_table.py --check` proves the table still matches the jar.

## What the biome source reproduces, and its one simplification

Reproduced exactly: the parameter intervals of the four biomes' rows (no
rounding: `f32` from the JSON's decimals), the row order (so ties resolve as the
jar's `findValue` does), and the six-axis distance.

**Simplification, stated:** the SDK gives a `BiomeSource` a quart cell and the
seed and **no climate sampler**, so the port cannot read the mod's seeded
climate noise. `runtime::climate_at` instead derives the climate from the cell
with a value noise (16-quad cells, one draw per axis) inside the four biomes'
covered ranges — continentalness -0.19..1.0, temperature -1.0..1.0, humidity
-1.0..1.0, erosion/depth/weirdness fixed at 0.0 — and the answer is *the
nearest of the slice's rows*. It is deterministic, allocations-free and
independent of the seed (as the brief's readout requires). Two consequences,
both real: biome regions do not follow the terrain's climate noise, and a cell
outside the slice's ranges answers the nearest covered biome. `Grid` precomputes
the 4,096 lattice answers once (4,884 rows × 4,096 cells) because `noise_biome`
is asked for every 4×4×4 cell of every chunk.

## The generator, and what is a reconstruction

The jar's generator id is `eternal_starlight:biome_based`, a Java type. The
campaign does not decompile, and the artefact says what it must do:

- its noise settings' `noise_router.final_density` is **`0.0`** and all other
  terrain fields are `0.0` too (measured in the jar's
  `worldgen/noise_settings/starlight.json`), so vanilla's noise fill cannot be
  the terrain: a constant non-positive density fills nothing;
- its `biome_data` registry gives each biome a **`height`** and **`variance`**
  (forest 65/14, dense forest 65/14, scrubland 60/10, dark swamp 61/5);
- `ESChunkGenerator` in `main` extends `NoiseBasedChunkGenerator` and adds only
  river fluid — an earlier shape than 0.9.1's, so it does not settle the 0.9.1
  algorithm.

So `BiomeBased` (the SDK's `ChunkGenerator`) builds each column from exactly the
artefact's numbers: the biome's `height` ± its `variance` times a seeded value
noise, grimstone below, water up to sea level 48, the material rule's surface
layers, its dark-swamp bands and its voidstone floor, and bedrock at `min_y`.
The `vanilla_stages` ask only for **features**, because the carried biomes'
feature entries are vanilla placed features; surface, carvers and structures are
this generator's own.

**Stated reconstruction** (not a translation): the noise shape (value noise,
32-block and 64-block cells), the bedrock layer (the rule's random 0..5 floor
simplified to `min_y`), and dark swamp's bands (the rule's random transitions
22..30 and 32..40 at their midpoints 26 and 40). `notes` for the next slice:
match the 0.9.1 class by reading its behaviour in game, not by guessing.

## Gaps (each with the artefact that forced it)

- **The other nineteen biomes.** The jar has 23 biome files; the slice carries
  four. The others' parameter rows are not in `src/table.rs`, so a cell outside
  the four answers the nearest covered biome.
- **Structures: all eight dropped.**
  `data/eternal_starlight/worldgen/template_pool/portal_ruins_forest.json:5`
  `"element_type": "eternal_starlight:single_pool"` and
  `…structure_set/cursed_garden.json:3` `"type": "eternal_starlight:landmark"`
  are Java types;
  `…worldgen/structure/cursed_garden.json:2` `"type":
  "eternal_starlight:cursed_garden"` and `stranghoul_den.json:2`
  `eternal_starlight:stranghoul_den` are Java structure types;
  the six `minecraft:jigsaw` structures (golem forge, five portal ruins) need
  out-of-slice blocks in their templates and a biome tag whose members are not
  in the slice — item 4's "only if its structure type is data-defined" is not
  met by any of them.
- **Carvers.** `worldgen/configured_carver/caves.json:2` `"type":
  "eternal_starlight:caves"` is a Java carver; the carried biomes name
  `minecraft:cave` instead (the closest data-defined carver), so caves are
  vanilla-shaped.
- **Features.** Every mod feature entry is a Java feature or placement type —
  `eternal_starlight:fallen_log`, `:spike`, `:speleothem`, `:lake`,
  `:block_patch`, `:pillar`, `:height_range_filter`, `:noise_boolean_selector`,
  `:avoid_structure`, `:swamp_water`, `:final_modification`, `:bouldershroom`,
  `:stellagmite` — and they place 24+ plants, 12+ ores and 15+ tree blocks per
  biome. They are dropped; the vanilla placed features the biomes also name
  (`minecraft:freeze_top_layer`, `patch_grass_savanna`, `patch_tall_grass`) are
  kept and run. 31/30/30/36 entries per biome were dropped (the carry prints the
  counts).
- **Mobs, items, menus, progression.** The biomes' `spawners` name Java entity
  types; they are dropped (empty `natural_mob_spawns`). The mod's 1,283 recipes
  and its items are out of the slice, so no carried block is craftable.
- **Client effects.** `dimension_type.starlight.json` `"effects":
  "eternal_starlight:special_effect"` (the dark starry sky) is client-only and
  has no `DimensionSettings` field; the level uses the Overworld sky. The
  biomes' custom particle (`eternal_starlight:starlight`) and music are dropped
  from their converted `effects`/`attributes`.
- **Fixed time value.** The jar sets `"fixed_time": 12900`; `DimensionSettings`
  has `has_fixed_time` only, so the level's sky is fixed at the engine's value,
  not 12900.
- **In-game run: not claimed.** No launcher exists at this pin, so the mod
  cannot be installed and `tools/e2e.sh` has never run. The script is written
  for a launcher-built game (`GAME=`, `CONTROL_GAME=`, `SELFTEST=1`) and its
  in-game half is this gap.
- **Performance of `build_terrain`** is unmeasured (no game); it writes every
  block of the column rather than batching sections.

## Adaptations (1.21.1 data → 26.3), from the pin's upgrade rules

The conversions follow `ports/_shared/pins/horizon-1d63e0a/crates/horizon-port-pack/src/upgrade/`
(`settings.rs`, `density.rs`, `registries.rs`), reimplemented in
`tools/carry_content.py` for the handful of files the slice moves:

- **Noise settings**: eight-field router; `initial_density_without_jaggedness`
  becomes `chunk_surface_level` (`interpolated`, cell 16×1) and the beardifier is
  added back to `final_density`; the surface rule moves to the
  `worldgen/material_rule` registry and is named by `material_rule`; aquifers and
  ore veins are dropped because 1.21.1 disabled both; the inline
  `shifted_noise` temperature/vegetation are the vanilla
  `minecraft:overworld/temperature|vegetation` functions they duplicate;
  `spawn_target` points become `{density-id: interval}` objects.
- **Density**: `y_clamped_gradient` → `gradient` (axis y);
  `flat_cache`/`cache_2d` → `cache`; `argument1`/`argument2` →
  `left`/`right`; block states `{"Name":…}` → their id.
- **Noise**: `firstOctave`/`amplitudes` → `base_octave`/`base_amplitude`.
- **Material rule**: `eternal_starlight:on_surface` → vanilla's
  `minecraft:above_preliminary_surface` (it is the same structural condition —
  the surface band above the preliminary surface — and the only Java condition
  in the rule); `minecraft:water` gains its codec defaults (offset 0, multiplier
  0, no stone depth); `biome_is` branches for biomes outside the slice are
  pruned (they can never match: those ids are not in `possible_biomes`).
- **Biomes**: `spawners`/`spawn_costs` → the
  `minecraft:gameplay/natural_mob_spawns` attribute; `carvers` map → array;
  colour ints → `#rrggbb`, with sky/fog in `attributes` and the rest in
  `effects`; `creature_spawn_probability` dropped (26.3's default).
- **Item models**: `models/item/<id>.json` → `items/<id>.json` with the
  `minecraft:model` node.
- **Loot**: 26.3's block shape (`condition: minecraft:tool/can_silk_touch`,
  `survives_explosion`), with the drop read from the jar's own table.

## Upstream defects fixed

**None found, and how it was checked:** every carried file was compared with its
26.3 counterpart in the vanilla data pack
(`/home/adlf/Desktop/horizon/.local/minecraft/26.3/jars/server-game.jar`,
`data/minecraft/worldgen/{biome/noise_settings/material_rule,…}`) for shape, and
the jar's own cross-references were checked: each carried block's drop exists
(`grimstone` → `cobbled_grimstone`, `nightfall_grass_block` → `nightfall_dirt`,
`fantasy_grass_block` → `nightfall_mud`, `voidstone` → `cobbled_voidstone`,
others → themselves, all in the nine); every material-rule block id is declared;
every biome id the carried rule names is in `possible_biomes`; every carried
feature id exists in 26.3's `placed_feature`. One inconsistency is **not** a
defect but a limitation the templates cannot express: the mod's `ESGrassBlock`
spreads to `nightfall_dirt`/`nightfall_mud`, while the `minecraft:grass_block`
template spreads to `minecraft:dirt`; that is recorded as a deviation for a
later slice (a block part), not "fixed" here.

## Verification (evidence)

Commands were run through `ports/tools/build-slot.sh` with `BUILD_LOCAL=1`
(SDK ports are local), `CARGO_INCREMENTAL=0`, the package's own `.horizon/sdk`
at the pin, and `TMPDIR` inside the package — the box's `/tmp` tmpfs was full
(`StorageFull` on a rustdoc temp file), which is an environment condition, not a
port defect. Logs are in `ports/_shared/evidence/eternal_starlight/`.

| Step | Command | Result |
|---|---|---|
| Compile + pure tests | `cargo test --offline` | **19 passed**, 0 failed, 0 ignored (`ladder-test.log`) |
| Formatting | `cargo fmt --check` (never `--all`) | clean (`ladder-fmt-clippy-test.log`) |
| Lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`ladder-fmt-clippy-test.log`) |
| Carried data | `python3 tools/carry_content.py --check` | `OK 70 files, 30297 bytes` |
| Parameter table | `python3 tools/gen_table.py --check` | `OK src/table.rs: 4884 rows` |
| Package build 1 | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/eternal_starlight.hmod`, exit 0 (`build-1.log`) |
| Package build 2 | the same again | byte-identical, exit 0 (`build-2.log`) |
| Registry check | `cargo horizon-mod check eternal_starlight.hmod` | `eternal_starlight 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)`, `65014 bytes, SHA-256 9568d626…8742, unsigned`, **exit 0** (`check-fits.log`) |
| What it uses | `cargo horizon-mod check … --json` | exit 0 (`check.json`); `capabilities: []` / `side: null` is what `--json` reports for a native package, not a defect |
| Fits the build | `cargo horizon-mod fits eternal_starlight.hmod --build nightly` | `0b14f8a  made for`, **exit 0** (`check-fits.log`) |

**Package** (`package.txt`): `ports/eternal_starlight/eternal_starlight.hmod`,
**65,014 B**, SHA-256
`9568d626b98917b9d8c2678e94d3dd563969b5724dc2dffb7628c4d463d78742`, **79
entries** (44 `assets/eternal_starlight/**`, 26 `data/eternal_starlight/**`,
`src/{lib,runtime,table}.rs`, `horizon-mod.toml`, `Cargo.toml`, `README.md`,
`LICENSE`, `LICENSE-NOTICE.md`, `uses.json`), 403,996 B uncompressed. **No
`Cargo.lock`** inside; `LICENSE` and `LICENSE-NOTICE.md` are.

**Not run: the in-game e2e.** `tools/e2e.sh` needs a game the launcher built with
the mod (`horizon-launcher native-build … --trust-native-code`), and no launcher
exists at this pin, so the script has never run and no in-game behaviour is
claimed. Its legs are written to be fail-capable (modded vs unmodified game, and
`SELFTEST=1` inverts the control assertion); the next slice runs it and records
`PASS` + `EXIT:0` before the port moves past `ported-partial`.
