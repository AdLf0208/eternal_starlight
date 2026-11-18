//! Eternal Starlight for Horizon, slice 1: the Starlight dimension exists, is
//! enterable and generates for four of its biomes.
//!
//! See `notes.md` for the scope, the jar inventory and the gaps. `runtime` holds
//! the port's pure logic and its tests, `table` the jar's parameter rows; this
//! file is the glue: the two registrations and the readout command.
//!
//! What the jar registers and what this port does with it:
//!
//! * `data/eternal_starlight/dimension/starlight.json` names the Java generator
//!   `eternal_starlight:biome_based`. Its class (`ESChunkGenerator`) extends
//!   vanilla's `NoiseBasedChunkGenerator` and only adds river fluid, so the
//!   dimension is registered over `minecraft:noise` with the mod's **own** noise
//!   settings (carried, converted to the 26.3 shape) and its own parameters.
//! * The biome source `eternal_starlight:multi_noise` is a Java type, so
//!   [`StarlightBiomes`] answers it in Rust over the carried parameter rows.
//! * The dimension type's fields are all expressible in `DimensionSettings`
//!   except the client effect id, which this slice records as a gap.

pub mod runtime;
pub mod table;

use std::sync::OnceLock;

use mod_api::{
    self, BiomeSource, BlockKey, ChunkAccess, ChunkGenerator, CommandArgumentValue,
    DimensionSettings, Flow, Hooks, RegisterCommandsEvent, Registrar, RegistrationContext,
    RunModCommandEvent, ServerHost, Text, VanillaStages,
};

/// The dimension the jar ships, and the level `/execute in` names.
pub const DIMENSION: &str = "eternal_starlight:starlight";
/// The mod's biome source id, registered below and named by the dimension.
pub const BIOME_SOURCE: &str = "eternal_starlight:multi_noise";
/// The jar's chunk generator id.
pub const GENERATOR: &str = "eternal_starlight:biome_based";
/// The carried noise settings (`data/eternal_starlight/worldgen/noise_settings/`).
pub const NOISE_SETTINGS: &str = "eternal_starlight:starlight";
/// The biome a readout falls back to when the rule places nothing.
pub const DEFAULT_BLOCK: &str = "eternal_starlight:grimstone";

/// The mod's biome source: the slice's parameter rows, answered from the
/// precomputed lattice grid. Stateless after the first call, deterministic, and
/// independent of the seed (the climate it is given is the slice's
/// simplification; see `runtime::climate_at`).
struct StarlightBiomes;

impl BiomeSource for StarlightBiomes {
    fn possible_biomes(&self) -> Vec<String> {
        table::BIOMES.iter().map(|id| (*id).to_owned()).collect()
    }

    fn noise_biome(&self, quart: [i32; 3], _seed: i64) -> u16 {
        grid().biome_at(quart)
    }
}

/// The jar's `eternal_starlight:biome_based` generator, for slice 1.
///
/// The 0.9.1 class is Java and this campaign does not decompile: what the artefact
/// states is the biome table's per-biome `height`/`variance` (`biome_data`), the
/// noise settings' `default_block` (grimstone), `sea_level` (48) and the material
/// rule's layers. The generator builds each column from exactly those: the biome's
/// height plus its variance times a seeded value noise, grimstone below, water up
/// to sea level, and the rule's surface, bands and voidstone floor. `notes.md`
/// records this as the slice's stated reconstruction and what a later slice must
/// do to match the Java class exactly.
struct BiomeBased {
    bedrock: BlockKey,
    grimstone: BlockKey,
    nightfall_dirt: BlockKey,
    nightfall_grass: BlockKey,
    fantasy_grass: BlockKey,
    nightfall_mud: BlockKey,
    packed_nightfall_mud: BlockKey,
    voidstone: BlockKey,
    water: BlockKey,
}

impl BiomeBased {
    /// The state each layer is written with, in [`runtime::Layer`] order, plus
    /// water. Resolved once per chunk, as `ChunkAccess::state` requires.
    fn states(&self, chunk: &dyn ChunkAccess) -> [mod_api::BlockState; 9] {
        [
            chunk.state(self.bedrock),
            chunk.state(self.grimstone),
            chunk.state(self.nightfall_dirt),
            chunk.state(self.nightfall_grass),
            chunk.state(self.fantasy_grass),
            chunk.state(self.nightfall_mud),
            chunk.state(self.packed_nightfall_mud),
            chunk.state(self.voidstone),
            chunk.state(self.water),
        ]
    }
}

impl ChunkGenerator for BiomeBased {
    fn build_terrain(&self, chunk: &mut dyn ChunkAccess) {
        let states = self.states(chunk);
        let pos = chunk.pos();
        let seed = chunk.seed();
        let min_y = chunk.min_y();
        let ceiling = min_y + chunk.height();
        let sea = runtime::SEA_LEVEL;
        let min_x = pos.x * 16;
        let min_z = pos.z * 16;
        for x in 0..16 {
            for z in 0..16 {
                let world_x = min_x + x;
                let world_z = min_z + z;
                let biome =
                    grid().biome(&runtime::climate_at([world_x >> 2, sea >> 2, world_z >> 2]));
                let top = runtime::column_height(biome, world_x, world_z, seed).min(ceiling - 1);
                for y in min_y..ceiling {
                    let state = if y == min_y {
                        states[0]
                    } else if y <= top {
                        match runtime::column_layer(biome, y, top) {
                            Some(layer) => states[layer.index() + 1],
                            None => states[1],
                        }
                    } else if top < sea && y < sea {
                        states[8]
                    } else {
                        continue;
                    };
                    chunk.set_block([world_x, y, world_z], state);
                }
            }
        }
    }

    /// The carried biomes' features are vanilla placed features, so the level
    /// asks for the feature stage; the surface, carvers and structures are this
    /// generator's own (the mod's carver and structures are Java types, recorded
    /// as gaps).
    fn vanilla_stages(&self) -> VanillaStages {
        VanillaStages {
            features: true,
            ..VanillaStages::default()
        }
    }
}

/// The lattice grid, built once for the process: 4,884 rows times 4,096 cells,
/// which is why `noise_biome` cannot scan the table per cell.
fn grid() -> &'static runtime::Grid {
    static GRID: OnceLock<runtime::Grid> = OnceLock::new();
    GRID.get_or_init(runtime::Grid::build)
}

/// The Starlight dimension type, from the jar's `dimension_type/starlight.json`,
/// with the generator Horizon's own noise generator runs.
fn settings() -> DimensionSettings {
    DimensionSettings {
        // "height": 384, "min_y": -64, "logical_height": 384
        min_y: -64,
        height: 384,
        logical_height: 384,
        // "has_skylight": true, "has_ceiling": false, "ambient_light": 0.0
        has_skylight: true,
        has_ceiling: false,
        ambient_light: 0.0,
        // The jar's own generator id, registered from [`BiomeBased`] below.
        generator: GENERATOR.to_owned(),
        noise_settings: NOISE_SETTINGS.to_owned(),
        biome: table::BIOMES[0].to_owned(),
        mod_biome_source: Some(BIOME_SOURCE.to_owned()),
        // "fixed_time": 12900 — the sky does not advance. `DimensionSettings`
        // has no fixed time value; notes.md records the difference.
        has_fixed_time: true,
        // "coordinate_scale": 1.0, "infiniburn": "#minecraft:infiniburn_overworld"
        coordinate_scale: 1.0,
        // "monster_spawn_light_level": uniform 0..=7,
        // "monster_spawn_block_light_limit": 7
        monster_spawn_light_level: (0, 7),
        monster_spawn_block_light_limit: 7,
        // "respawn_anchor_works": true, "has_raids": false, "piglin_safe": false,
        // "ultrawarm": false, "natural": true, "bed_works": true
        respawn_anchor_works: true,
        can_start_raid: false,
        nether_portal_spawns_piglin: false,
        piglins_zombify: false,
        water_evaporates: false,
        fast_lava: false,
        // The noise settings' own sea level.
        sea_level: Some(48),
        // "effects": "eternal_starlight:special_effect" is client-only and out
        // of this slice; the sky stays the Overworld's.
        skybox: "overworld".to_owned(),
        bed_can_sleep: "when_dark".to_owned(),
        bed_can_set_spawn: "always".to_owned(),
        day_cycle: "overworld".to_owned(),
        ..DimensionSettings::default()
    }
}

/// The mod has no state of its own: the grid is a process-wide `OnceLock`.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Once, at startup: the biome source and the dimension, in that order, so
    /// the dimension can name the source.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        r.register_biome_source(BIOME_SOURCE, StarlightBiomes);
        let blocks = BiomeBased {
            bedrock: r.block_key("minecraft:bedrock"),
            grimstone: r.block_key("eternal_starlight:grimstone"),
            nightfall_dirt: r.block_key("eternal_starlight:nightfall_dirt"),
            nightfall_grass: r.block_key("eternal_starlight:nightfall_grass_block"),
            fantasy_grass: r.block_key("eternal_starlight:fantasy_grass_block"),
            nightfall_mud: r.block_key("eternal_starlight:nightfall_mud"),
            packed_nightfall_mud: r.block_key("eternal_starlight:packed_nightfall_mud"),
            voidstone: r.block_key("eternal_starlight:voidstone"),
            water: r.block_key("minecraft:water"),
        };
        r.register_chunk_generator(GENERATOR, blocks);
        r.register_dimension("starlight", settings());
    }

    /// `/eternal_starlight status [<x> <y> <z>]`: the registered dimension, the
    /// biome count and this cell's answer.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(mod_api::Command::new(
            "eternal_starlight status [<x:integer> <y:integer> <z:integer>]",
        ));
    }

    /// Answer `status`, and let every other usage through.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        const NAME: &str = "eternal_starlight";
        if event.command.literals.first().map(String::as_str) != Some(NAME) {
            return Flow::Continue;
        }
        if event.command.literals.get(1).map(String::as_str) != Some("status") {
            return Flow::Continue;
        }
        let block = match (
            integer(event.command, "x"),
            integer(event.command, "y"),
            integer(event.command, "z"),
        ) {
            (Some(x), Some(y), Some(z)) => [x, y, z],
            // No position: the invocation's own, as `/execute positioned` set it.
            _ => event.command.position.map(|value| value.floor() as i32),
        };
        let quart = [block[0] >> 2, block[1] >> 2, block[2] >> 2];
        let climate = runtime::climate_at(quart);
        let biome = grid().biome(&climate);
        let index = usize::from(biome);
        let surface =
            runtime::surface_layer(biome, 0, false).map_or(DEFAULT_BLOCK, runtime::Layer::id);
        let line = runtime::status_line(
            DIMENSION,
            block,
            table::BIOMES[index],
            table::BIOMES.len(),
            surface,
        );
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}

/// The `integer` argument `name`, if the command carried it.
fn integer(command: &mod_api::CommandInvocation, name: &str) -> Option<i32> {
    match command.argument(name) {
        Some(CommandArgumentValue::Integer(value)) => Some(*value),
        _ => None,
    }
}
