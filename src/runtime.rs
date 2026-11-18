//! Eternal Starlight's worldgen logic: the parameter table's nearest-point
//! answer, the position-derived climate the SDK's `BiomeSource` is given, and
//! the surface each slice biome's material rule lays down.
//!
//! The jar's `eternal_starlight:multi_noise` source is a Java type
//! (`ESBiomeSource`), so the port answers from Rust: `nearest` is vanilla's
//! `Climate.ParameterList.findValue` distance over the rows `table.rs` copies
//! out of the jar, and `climate_at` supplies the climate the game would have
//! sampled. The SDK's `BiomeSource::noise_biome` hands us a quart cell and a
//! seed and nothing else — there is no climate sampler on the mod side — so the
//! climate is a deterministic value noise of the cell, independent of the seed.
//! That is this slice's stated simplification; `notes.md` carries it.

/// The six climate axes, in the order a [`Row`]'s axes are stored.
pub const AXES: usize = 6;

/// One row of the jar's `biome_data_parameters` table: the biome it selects and
/// one interval index per axis (see [`INTERVALS`]). `offset` is the row's own
/// `Climate.ParameterPoint.offset`; every one of the slice's rows is `0.0` in
/// the jar, which `tools/gen_table.py` refuses to copy otherwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    /// Index into [`BIOMES`].
    pub biome: u8,
    /// Interval index per axis, in [`AXES`] order.
    pub axes: [u8; AXES],
    /// The point's `offset`, 0 to 1.
    pub offset: f32,
}

/// A sampled climate, as vanilla's `Climate.TargetPoint` carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    /// `temperature`, -2 to 2.
    pub temperature: f32,
    /// `humidity`, -2 to 2.
    pub humidity: f32,
    /// `continentalness`, -2 to 2.
    pub continentalness: f32,
    /// `erosion`, -2 to 2.
    pub erosion: f32,
    /// `depth`, -2 to 2.
    pub depth: f32,
    /// `weirdness`, -2 to 2.
    pub weirdness: f32,
}

/// `Climate.Parameter.distance`: zero inside the interval, otherwise how far
/// outside it `value` is.
pub fn axis_distance(interval: (f32, f32), value: f32) -> f32 {
    let (min, max) = interval;
    if value < min {
        min - value
    } else if value > max {
        value - max
    } else {
        0.0
    }
}

/// `Climate.ParameterPoint.distance`: one [`axis_distance`] per axis plus the
/// row's own `offset`.
pub fn distance(row: &Row, climate: &Climate) -> f32 {
    let values = [
        climate.temperature,
        climate.humidity,
        climate.continentalness,
        climate.erosion,
        climate.depth,
        climate.weirdness,
    ];
    let mut total = row.offset;
    for (axis, value) in values.iter().enumerate() {
        let interval = super::table::INTERVALS[axis][row.axes[axis] as usize];
        total += axis_distance(interval, *value);
    }
    total
}

/// The row whose climate is nearest `climate`, first in table order on a tie:
/// vanilla's `Climate.ParameterList.findValue`.
pub fn nearest(climate: &Climate) -> u16 {
    let mut best = 0u16;
    let mut best_distance = f32::INFINITY;
    for (index, row) in super::table::ROWS.iter().enumerate() {
        let candidate = distance(row, climate);
        if candidate < best_distance {
            best_distance = candidate;
            best = index as u16;
        }
    }
    best
}

/// How many lattice steps each climate axis is quantised to.
pub const LATTICE: usize = 16;

/// The interval each varying axis covers in slice 1: the union of the four
/// biomes' own ranges, taken from the jar's table. Erosion, depth and weirdness
/// stay at 0, which every one of the four covers.
pub const CONTINENTALNESS: (f32, f32) = (-0.19, 1.0);
/// The temperature range slice 1 covers.
pub const TEMPERATURE: (f32, f32) = (-1.0, 1.0);
/// The humidity range slice 1 covers.
pub const HUMIDITY: (f32, f32) = (-1.0, 1.0);

/// The climate of one lattice cell, by its three axis steps.
pub fn lattice_climate(continentalness: usize, temperature: usize, humidity: usize) -> Climate {
    Climate {
        temperature: axis_value(TEMPERATURE, temperature),
        humidity: axis_value(HUMIDITY, humidity),
        continentalness: axis_value(CONTINENTALNESS, continentalness),
        erosion: 0.0,
        depth: 0.0,
        weirdness: 0.0,
    }
}

/// Where lattice step `step` sits on `range`; the ends are exactly the range's.
fn axis_value(range: (f32, f32), step: usize) -> f32 {
    let t = step.min(LATTICE - 1) as f32 / (LATTICE - 1) as f32;
    range.0 * (1.0 - t) + range.1 * t
}

/// The lattice step a value of `range` is nearest to.
fn axis_step(range: (f32, f32), value: f32) -> usize {
    let t =
        ((value.clamp(range.0, range.1) - range.0) / (range.1 - range.0)) * (LATTICE - 1) as f32;
    (t.round() as usize).min(LATTICE - 1)
}

/// The climate at quart cell `quart` (`[x, y, z]`, each a block position >> 2).
///
/// A value noise of the cell's `x` and `z` over [`NOISE_CELL`]-sized cells, one
/// draw per axis: the same cell always gives the same climate, and the seed is
/// not an input because the SDK gives the source no climate sampler to read the
/// seeded noise from.
pub fn climate_at(quart: [i32; 3]) -> Climate {
    let [x, _, z] = quart;
    Climate {
        temperature: map_axis(TEMPERATURE, noise(x, z, NOISE_CELL, SALT_TEMPERATURE)),
        humidity: map_axis(HUMIDITY, noise(x, z, NOISE_CELL, SALT_HUMIDITY)),
        continentalness: map_axis(
            CONTINENTALNESS,
            noise(x, z, NOISE_CELL, SALT_CONTINENTALNESS),
        ),
        erosion: 0.0,
        depth: 0.0,
        weirdness: 0.0,
    }
}

/// Where `t` (0 to 1) sits on `range`; the ends are exactly the range's.
fn map_axis(range: (f32, f32), t: f32) -> f32 {
    range.0 * (1.0 - t) + range.1 * t
}

/// Quart cells per value-noise cell: 16 quads is 64 blocks.
pub const NOISE_CELL: i32 = 16;

const SALT_TEMPERATURE: u64 = 0x7e4a_1c6b_90d3_f215;
const SALT_HUMIDITY: u64 = 0x1b93_5f0c_2a7e_64d8;
const SALT_CONTINENTALNESS: u64 = 0xc05a_9e37_4b16_88f2;

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Quads per terrain-noise cell: 8 quads is 32 blocks.
pub const TERRAIN_CELL: i32 = 8;

/// A seeded value noise in -1..1 over [`TERRAIN_CELL`] quads: the height
/// variance's draw. The seed is the level's, so a world's hills are its own.
fn seeded_noise(x: i32, z: i32, seed: i64) -> f32 {
    let salt = (seed as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(17)
        ^ 0x0005_deec_e66d;
    2.0 * noise(x, z, TERRAIN_CELL, salt) - 1.0
}

/// A two-dimensional value noise in 0..1 over cells `cell` quads wide.
fn noise(x: i32, z: i32, cell: i32, salt: u64) -> f32 {
    let x0 = x.div_euclid(cell);
    let z0 = z.div_euclid(cell);
    let fx = smooth(x.rem_euclid(cell) as f32 / cell as f32);
    let fz = smooth(z.rem_euclid(cell) as f32 / cell as f32);
    let corners = [
        hash(x0, z0, salt),
        hash(x0 + 1, z0, salt),
        hash(x0, z0 + 1, salt),
        hash(x0 + 1, z0 + 1, salt),
    ];
    lerp(
        lerp(corners[0], corners[1], fx),
        lerp(corners[2], corners[3], fx),
        fz,
    )
}

/// A lattice corner's pseudo-random value in 0..1 (a splitmix64 finaliser over
/// the two coordinates and the axis's salt).
fn hash(x: i32, z: i32, salt: u64) -> f32 {
    let mut h = salt ^ 0x9e37_79b9_7f4a_7c15;
    for value in [x as u32 as u64, z as u32 as u64] {
        h = (h ^ value.wrapping_mul(0x1000_0000_01b3)).rotate_left(27);
        h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h >> 40) as f32 / 16_777_216.0
}

/// The untaken [`nearest`] answer for a quart cell.
pub fn nearest_at(quart: [i32; 3]) -> u16 {
    nearest(&climate_at(quart))
}

/// Every lattice cell's row, built once: `noise_biome` is asked for 4,096 cells
/// of every chunk, so the exact scan runs here instead of per cell.
#[derive(Clone, Debug)]
pub struct Grid {
    cells: Vec<u8>,
}

impl Grid {
    /// Answer every lattice cell with [`nearest`].
    pub fn build() -> Self {
        let mut cells = Vec::with_capacity(LATTICE * LATTICE * LATTICE);
        for continentalness in 0..LATTICE {
            for temperature in 0..LATTICE {
                for humidity in 0..LATTICE {
                    let row = nearest(&lattice_climate(continentalness, temperature, humidity));
                    cells.push(super::table::ROWS[row as usize].biome);
                }
            }
        }
        Self { cells }
    }

    /// The biome index of a climate, by its nearest lattice cell.
    pub fn biome(&self, climate: &Climate) -> u8 {
        let continentalness = axis_step(CONTINENTALNESS, climate.continentalness);
        let temperature = axis_step(TEMPERATURE, climate.temperature);
        let humidity = axis_step(HUMIDITY, climate.humidity);
        let index = (continentalness * LATTICE + temperature) * LATTICE + humidity;
        self.cells[index]
    }

    /// The biome index at a quart cell.
    pub fn biome_at(&self, quart: [i32; 3]) -> u16 {
        u16::from(self.biome(&climate_at(quart)))
    }
}

/// Which layer of a surface a depth is: `0` is the top block, `1..=3` the
/// filler the material rule's `add_surface_depth` branch covers.
pub const FILLER_DEPTH: i32 = 3;

/// The noise settings' `sea_level`.
pub const SEA_LEVEL: i32 = 48;

/// Each slice biome's `biome_data` `height` and `variance`, in [`BIOMES`] order,
/// from the jar's `data/eternal_starlight/eternal_starlight/biome_data/`.
pub const BIOME_HEIGHT: [(i32, i32); 4] = [(65, 14), (65, 14), (60, 10), (61, 5)];

/// One block a slice column is made of: the carried settings' `default_block`,
/// the material rule's surface layers and its later branches, and the bedrock
/// floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// `minecraft:bedrock`, the rule's floor.
    Bedrock,
    /// `eternal_starlight:grimstone`, the noise settings' `default_block`.
    Grimstone,
    /// `eternal_starlight:nightfall_dirt`.
    NightfallDirt,
    /// `eternal_starlight:nightfall_grass_block`.
    NightfallGrass,
    /// `eternal_starlight:fantasy_grass_block`.
    FantasyGrass,
    /// `eternal_starlight:nightfall_mud`.
    NightfallMud,
    /// `eternal_starlight:packed_nightfall_mud`.
    PackedNightfallMud,
    /// `eternal_starlight:voidstone`.
    Voidstone,
}

impl Layer {
    /// The block id this layer places.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bedrock => "minecraft:bedrock",
            Self::Grimstone => "eternal_starlight:grimstone",
            Self::NightfallDirt => "eternal_starlight:nightfall_dirt",
            Self::NightfallGrass => "eternal_starlight:nightfall_grass_block",
            Self::FantasyGrass => "eternal_starlight:fantasy_grass_block",
            Self::NightfallMud => "eternal_starlight:nightfall_mud",
            Self::PackedNightfallMud => "eternal_starlight:packed_nightfall_mud",
            Self::Voidstone => "eternal_starlight:voidstone",
        }
    }

    /// This layer's place in the generator's state list, `Bedrock` first.
    pub const fn index(self) -> usize {
        match self {
            Self::Bedrock => 0,
            Self::Grimstone => 1,
            Self::NightfallDirt => 2,
            Self::NightfallGrass => 3,
            Self::FantasyGrass => 4,
            Self::NightfallMud => 5,
            Self::PackedNightfallMud => 6,
            Self::Voidstone => 7,
        }
    }
}

/// The block the slice's material rule puts at `depth` below the surface of
/// `biome`, or `None` where the carried rule places nothing and the noise
/// settings' `default_block` stands.
///
/// The rule is `data/eternal_starlight/worldgen/material_rule/starlight.json`:
/// the top layer under water is the biome's grass, on dry land the `water`
/// condition fails and the filler branch's depth-0 row decides. `tools/carry_content.py`
/// proves this table against that file.
pub fn surface_layer(biome: u8, depth: i32, underwater: bool) -> Option<Layer> {
    if !(0..=FILLER_DEPTH).contains(&depth) {
        return None;
    }
    let (dry, wet, filler) = match usize::from(biome) {
        0..=2 => (
            Layer::NightfallDirt,
            Layer::NightfallGrass,
            Layer::NightfallDirt,
        ),
        3 => (
            Layer::NightfallMud,
            Layer::FantasyGrass,
            Layer::NightfallMud,
        ),
        _ => return None,
    };
    Some(if depth > 0 {
        filler
    } else if underwater {
        wet
    } else {
        dry
    })
}

/// The rule's later branches for a solid block at `y`: the voidstone floor and
/// dark swamp's two mud bands. `None` leaves the surface layer or the default
/// block standing.
pub fn deep_layer(biome: u8, y: i32) -> Option<Layer> {
    // `vertical_gradient` from absolute 0 to 8: voidstone at and below 8.
    if y <= 8 {
        return Some(Layer::Voidstone);
    }
    if biome == 3 {
        // The rule's random bands at 22..=30 and 32..=40, at their midpoints.
        if y <= 26 {
            return Some(Layer::PackedNightfallMud);
        }
        if y <= 40 {
            return Some(Layer::NightfallMud);
        }
    }
    None
}

/// The topmost solid block of a column: the biome's `biome_data` height plus its
/// variance times a seeded value noise in -1..1.
pub fn column_height(biome: u8, x: i32, z: i32, seed: i64) -> i32 {
    let Some(&(height, variance)) = BIOME_HEIGHT.get(usize::from(biome)) else {
        return SEA_LEVEL;
    };
    let noise = seeded_noise(x, z, seed);
    height + (noise * variance as f32).round() as i32
}

/// The block of the solid part of a column at `y`, whose topmost solid block is
/// `top`: the material rule's layers in the order the jar applies them, so the
/// voidstone floor and the mud bands win over the surface layers.
pub fn column_layer(biome: u8, y: i32, top: i32) -> Option<Layer> {
    if y > top {
        return None;
    }
    deep_layer(biome, y).or_else(|| surface_layer(biome, top - y, top < SEA_LEVEL - 1))
}

/// The status line `/eternal_starlight status` answers with.
pub fn status_line(
    dimension: &str,
    block: [i32; 3],
    biome: &str,
    biome_count: usize,
    surface: &str,
) -> String {
    let [x, y, z] = block;
    format!(
        "{dimension}: {biome_count} biomes; block {x} {y} {z} (cell {} {} {}) is {biome}; \
         its dry surface is {surface}",
        x >> 2,
        y >> 2,
        z >> 2
    )
}

#[cfg(test)]
mod tests {
    use super::super::table::{BIOMES, INTERVALS, ROWS};
    use super::*;

    fn interval(axis: usize, row: &Row) -> (f32, f32) {
        INTERVALS[axis][row.axes[axis] as usize]
    }

    #[test]
    fn a_value_inside_an_interval_is_at_no_distance() {
        assert_eq!(axis_distance((-0.5, 0.5), 0.0), 0.0);
        assert_eq!(axis_distance((-0.5, 0.5), -0.5), 0.0);
        assert_eq!(axis_distance((0.0, 0.0), 0.0), 0.0);
    }

    #[test]
    fn a_value_outside_an_interval_is_as_far_outside_as_it_is() {
        assert_eq!(axis_distance((-0.5, 0.5), -0.75), 0.25);
        assert_eq!(axis_distance((-0.5, 0.5), 1.0), 0.5);
        assert_eq!(axis_distance((0.2, 0.4), 1.0), 0.6);
    }

    #[test]
    fn the_distance_is_the_sum_of_the_axes_plus_the_offset() {
        let row = Row {
            biome: 0,
            axes: [0; AXES],
            offset: 0.0,
        };
        let mut climate = Climate {
            temperature: 0.0,
            humidity: 0.0,
            continentalness: 0.0,
            erosion: 0.0,
            depth: 0.0,
            weirdness: 0.0,
        };
        // Every interval of the first row is the first interval of its axis;
        // pick a climate far outside all of them so the sum is visible.
        let expected: f32 = (0..AXES)
            .map(|axis| axis_distance(interval(axis, &row), -2.0))
            .sum();
        climate.temperature = -2.0;
        climate.humidity = -2.0;
        climate.continentalness = -2.0;
        climate.erosion = -2.0;
        climate.depth = -2.0;
        climate.weirdness = -2.0;
        assert_eq!(distance(&row, &climate), expected);
        let offset = Row {
            offset: 0.25,
            ..row
        };
        assert_eq!(distance(&offset, &climate), expected + 0.25);
    }

    #[test]
    fn the_table_is_the_jar_rows_for_the_four_slice_biomes() {
        assert_eq!(ROWS.len(), 4884);
        assert_eq!(BIOMES.len(), 4);
        let mut counts = [0usize; 4];
        for row in ROWS.iter() {
            counts[usize::from(row.biome)] += 1;
            for axis in 0..AXES {
                let (min, max) = interval(axis, row);
                assert!(min <= max, "{min} > {max}");
                assert!((-2.0..=2.0).contains(&min), "{min} out of range");
                assert!((-2.0..=2.0).contains(&max), "{max} out of range");
            }
        }
        assert_eq!(counts, [1350, 486, 1242, 1806]);
    }

    #[test]
    fn the_biome_ids_are_the_slice() {
        assert_eq!(
            BIOMES,
            [
                "eternal_starlight:starlight_forest",
                "eternal_starlight:starlight_dense_forest",
                "eternal_starlight:glimmer_scrubland",
                "eternal_starlight:dark_swamp",
            ]
        );
    }

    #[test]
    fn nearest_answers_the_row_whose_intervals_contain_the_climate() {
        // The first row's own interval centre is inside it, so the scan cannot
        // do better than 0 and the first such row wins.
        let row = ROWS[0];
        let climate = Climate {
            temperature: midpoint(interval(0, &row)),
            humidity: midpoint(interval(1, &row)),
            continentalness: midpoint(interval(2, &row)),
            erosion: midpoint(interval(3, &row)),
            depth: midpoint(interval(4, &row)),
            weirdness: midpoint(interval(5, &row)),
        };
        let answer = nearest(&climate);
        assert_eq!(distance(&ROWS[answer as usize], &climate), 0.0);
        assert_eq!(answer, 0);
    }

    #[test]
    fn nearest_is_the_smallest_distance_in_the_table() {
        for cell in [[0, 0, 0], [13, 7, 91], [-40, 3, 22], [256, 0, -256]] {
            let climate = climate_at(cell);
            let answer = nearest(&climate);
            let best = distance(&ROWS[answer as usize], &climate);
            for row in ROWS.iter() {
                assert!(
                    distance(row, &climate) >= best,
                    "{cell:?}: {row:?} beats the answer"
                );
            }
        }
    }

    fn midpoint((min, max): (f32, f32)) -> f32 {
        (min + max) / 2.0
    }

    /// One grid for every test: building it is the port's startup cost, and it
    /// is 4,884 rows times 4,096 cells.
    fn shared_grid() -> &'static Grid {
        static GRID: std::sync::OnceLock<Grid> = std::sync::OnceLock::new();
        GRID.get_or_init(Grid::build)
    }

    #[test]
    fn a_lattice_cell_sits_inside_its_axis_range() {
        for step in 0..LATTICE {
            let climate = lattice_climate(step, LATTICE - 1 - step, step);
            assert!((CONTINENTALNESS.0..=CONTINENTALNESS.1).contains(&climate.continentalness));
            assert!((TEMPERATURE.0..=TEMPERATURE.1).contains(&climate.temperature));
            assert!((HUMIDITY.0..=HUMIDITY.1).contains(&climate.humidity));
            assert_eq!(climate.erosion, 0.0);
            assert_eq!(climate.depth, 0.0);
            assert_eq!(climate.weirdness, 0.0);
        }
        assert_eq!(lattice_climate(0, 0, 0).continentalness, CONTINENTALNESS.0);
        assert_eq!(
            lattice_climate(LATTICE - 1, LATTICE - 1, LATTICE - 1).continentalness,
            CONTINENTALNESS.1
        );
    }

    #[test]
    fn a_quart_cell_always_gives_the_same_climate() {
        let cell = [17, -3, 42];
        assert_eq!(climate_at(cell), climate_at(cell));
        assert_ne!(climate_at([0, 0, 0]), climate_at([64, 0, 64]));
    }

    #[test]
    fn the_grid_answers_the_nearest_row_of_its_lattice_cell() {
        let grid = shared_grid();
        for continentalness in [0, 5, LATTICE - 1] {
            for temperature in [0, 9, LATTICE - 1] {
                for humidity in [0, 3, LATTICE - 1] {
                    let climate = lattice_climate(continentalness, temperature, humidity);
                    assert_eq!(
                        grid.biome(&climate),
                        ROWS[nearest(&climate) as usize].biome,
                        "cell {continentalness},{temperature},{humidity}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_grid_covers_every_slice_biome() {
        let grid = shared_grid();
        let mut seen = [false; 4];
        for continentalness in 0..LATTICE {
            for temperature in 0..LATTICE {
                for humidity in 0..LATTICE {
                    let climate = lattice_climate(continentalness, temperature, humidity);
                    seen[usize::from(grid.biome(&climate))] = true;
                }
            }
        }
        assert_eq!(seen, [true; 4], "a slice biome is never selected");
    }

    #[test]
    fn the_grid_is_a_pure_function_of_the_cell() {
        let grid = shared_grid();
        for cell in [[0, 0, 0], [7, 60, -7], [-129, -8, 300]] {
            assert_eq!(
                grid.biome_at(cell),
                grid.biome_at(cell),
                "the grid changed between two reads at {cell:?}"
            );
            assert_eq!(
                u16::from(
                    ROWS[nearest(&lattice_climate(
                        axis_step(CONTINENTALNESS, climate_at(cell).continentalness),
                        axis_step(TEMPERATURE, climate_at(cell).temperature),
                        axis_step(HUMIDITY, climate_at(cell).humidity),
                    )) as usize]
                        .biome
                ),
                grid.biome_at(cell),
                "the exact scan and the grid disagree at {cell:?}"
            );
        }
    }

    #[test]
    fn a_dry_top_is_the_biomes_own_surface_and_its_filler_follows() {
        // starlight_forest (0) and dark_swamp (3): the rule's two branches.
        assert_eq!(surface_layer(0, 0, false), Some(Layer::NightfallDirt));
        assert_eq!(surface_layer(0, 0, true), Some(Layer::NightfallGrass));
        assert_eq!(
            surface_layer(0, FILLER_DEPTH, false),
            Some(Layer::NightfallDirt)
        );
        assert_eq!(surface_layer(3, 0, false), Some(Layer::NightfallMud));
        assert_eq!(surface_layer(3, 0, true), Some(Layer::FantasyGrass));
        assert_eq!(
            surface_layer(3, FILLER_DEPTH, true),
            Some(Layer::NightfallMud)
        );
        // Below the filler the rule places nothing and grimstone (the noise
        // settings' default block) stands.
        assert_eq!(surface_layer(0, FILLER_DEPTH + 1, false), None);
        assert_eq!(surface_layer(9, 0, false), None);
    }

    #[test]
    fn the_deep_branches_are_the_voidstone_floor_and_the_swamp_bands() {
        // The voidstone branch is `absolute` 0 to 8, over every biome.
        assert_eq!(deep_layer(0, 8), Some(Layer::Voidstone));
        assert_eq!(deep_layer(0, 9), None);
        assert_eq!(deep_layer(3, -64), Some(Layer::Voidstone));
        // dark_swamp's bands: packed mud in 22..=30, mud in 32..=40, the rule's
        // random transitions at their midpoints.
        assert_eq!(deep_layer(3, 40), Some(Layer::NightfallMud));
        assert_eq!(deep_layer(3, 26), Some(Layer::PackedNightfallMud));
        assert_eq!(deep_layer(3, 41), None);
        assert_eq!(deep_layer(0, 40), None);
    }

    #[test]
    fn a_column_is_grimstone_under_its_layers_and_voidstone_at_the_floor() {
        let top = 70;
        assert_eq!(column_layer(0, top, top), Some(Layer::NightfallDirt));
        assert_eq!(column_layer(0, top - 1, top), Some(Layer::NightfallDirt));
        assert_eq!(column_layer(0, top - 4, top), None);
        assert_eq!(column_layer(0, 8, top), Some(Layer::Voidstone));
        assert_eq!(column_layer(3, 35, top), Some(Layer::NightfallMud));
        assert_eq!(column_layer(3, 25, top), Some(Layer::PackedNightfallMud));
        assert_eq!(column_layer(0, top + 1, top), None);
    }

    #[test]
    fn a_columns_height_is_its_biomes_height_within_its_variance() {
        assert_eq!(BIOME_HEIGHT.len(), BIOMES.len());
        for (biome, (height, variance)) in BIOME_HEIGHT.iter().enumerate() {
            for cell in [[0, 0], [37, -19], [-500, 900]] {
                let actual = column_height(biome as u8, cell[0], cell[1], 20261105);
                assert!(
                    (height - variance..=height + variance).contains(&actual),
                    "biome {biome} at {cell:?}: {actual} outside {height}±{variance}"
                );
            }
        }
        // A different seed moves the noise, not the range.
        let a = column_height(0, 11, 13, 1);
        let b = column_height(0, 11, 13, 2);
        assert!((41..=79).contains(&a) && (41..=79).contains(&b));
    }

    #[test]
    fn a_columns_height_is_the_same_for_the_same_seed_and_position() {
        assert_eq!(
            column_height(3, 100, -100, 7),
            column_height(3, 100, -100, 7)
        );
    }

    #[test]
    fn every_layer_has_its_own_id_and_its_own_state_slot() {
        let layers = [
            Layer::Bedrock,
            Layer::Grimstone,
            Layer::NightfallDirt,
            Layer::NightfallGrass,
            Layer::FantasyGrass,
            Layer::NightfallMud,
            Layer::PackedNightfallMud,
            Layer::Voidstone,
        ];
        for (slot, layer) in layers.iter().enumerate() {
            assert_eq!(layer.index(), slot);
            assert!(!layer.id().is_empty());
        }
        for (a, first) in layers.iter().enumerate() {
            for second in &layers[a + 1..] {
                assert_ne!(first.id(), second.id());
            }
        }
    }

    #[test]
    fn the_status_line_names_the_dimension_biome_count_and_answer() {
        assert_eq!(
            status_line(
                "eternal_starlight:starlight",
                [10, 70, -4],
                "eternal_starlight:dark_swamp",
                4,
                "eternal_starlight:nightfall_mud",
            ),
            "eternal_starlight:starlight: 4 biomes; block 10 70 -4 (cell 2 17 -1) is \
             eternal_starlight:dark_swamp; its dry surface is eternal_starlight:nightfall_mud"
        );
    }
}
