# Eternal Starlight — a Horizon port

**Eternal Starlight** is a Minecraft mod. Adds the magical and mysterious starlight dimension. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [LeoMinecraftModding/eternal-starlight](https://github.com/LeoMinecraftModding/eternal-starlight) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Eternal Starlight ([source](https://github.com/LeoMinecraftModding/eternal-starlight)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `eternal_starlight.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- its noise settings' `noise_router.final_density` is **`0.0`** and all other
- its `biome_data` registry gives each biome a **`height`** and **`variance`**
- `ESChunkGenerator` in `main` extends `NoiseBasedChunkGenerator` and adds only

## Not yet ported

- **The other nineteen biomes.** The jar has 23 biome files; the slice carries
- **Structures: all eight dropped.**
- **Carvers.** `worldgen/configured_carver/caves.json:2` `"type":
- **Features.** Every mod feature entry is a Java feature or placement type —
- **Mobs, items, menus, progression.** The biomes' `spawners` name Java entity
- **Client effects.** `dimension_type.starlight.json` `"effects":
- **Fixed time value.** The jar sets `"fixed_time": 12900`; `DimensionSettings`
- **In-game run: not claimed.** No launcher exists at this pin, so the mod

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `eternal_starlight.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Eternal Starlight was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/LeoMinecraftModding/eternal-starlight

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run eternal_starlight` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/eternal_starlight/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
