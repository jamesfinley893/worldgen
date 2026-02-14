use crate::params::GenerationParams;
use crate::state::{Biome, WorldState};

const DIRS_8: [(isize, isize); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

pub fn run(state: &mut WorldState, params: &GenerationParams) {
    for y in 0..state.height {
        for x in 0..state.width {
            let elev = *state.elevation.get(x, y);
            let rain = *state.rainfall.get(x, y);
            let temp = *state.temperature.get(x, y);
            let wet = wetness(state, x, y, params.biomes.wetness_weight);

            let biome = classify(
                temp,
                rain,
                elev,
                wet,
                state.params.base.sea_level,
                *state.lake_id.get(x, y) > 0,
            );
            *state.biome.get_mut(x, y) = biome;

            let fertility =
                (rain * 0.7 + wet * 0.2 + (1.0 - (temp - 18.0).abs() / 40.0) * 0.1).clamp(0.0, 1.0);
            *state.fertility.get_mut(x, y) = fertility;
        }
    }

    for _ in 0..params.biomes.smoothing_passes {
        smooth_biomes(state);
    }
}

fn wetness(state: &WorldState, x: usize, y: usize, wetness_weight: f32) -> f32 {
    let q = *state.accumulation.get(x, y);
    let qn = (q.log10() / 4.0).clamp(0.0, 1.0);
    (1.0 - wetness_weight) * *state.rainfall.get(x, y) + wetness_weight * qn
}

fn classify(temp_c: f32, rain: f32, elev: f32, wet: f32, sea_level: f32, is_lake: bool) -> Biome {
    if elev <= sea_level {
        return Biome::Ocean;
    }
    if is_lake {
        return Biome::Lake;
    }
    if wet > 0.85 {
        return Biome::Wetland;
    }
    if elev > sea_level + 0.28 {
        return Biome::Alpine;
    }

    if temp_c < -8.0 {
        if rain < 0.18 {
            Biome::PolarDesert
        } else {
            Biome::Tundra
        }
    } else if temp_c < 4.0 {
        if rain > 0.35 {
            Biome::BorealForest
        } else {
            Biome::Tundra
        }
    } else if temp_c < 15.0 {
        if rain < 0.2 {
            Biome::TemperateGrassland
        } else if rain < 0.45 {
            Biome::Mediterranean
        } else {
            Biome::TemperateForest
        }
    } else if rain < 0.16 {
        Biome::HotDesert
    } else if rain < 0.38 {
        Biome::Savanna
    } else if rain < 0.62 {
        Biome::TropicalSeasonalForest
    } else {
        Biome::TropicalRainforest
    }
}

fn smooth_biomes(state: &mut WorldState) {
    let mut out = state.biome.clone();

    for y in 1..state.height.saturating_sub(1) {
        for x in 1..state.width.saturating_sub(1) {
            let center = *state.biome.get(x, y);
            if matches!(center, Biome::Ocean | Biome::Lake) {
                continue;
            }

            let mut counts = [0u8; 14];
            counts[usize::from(center.as_u8())] += 1;

            for (dx, dy) in DIRS_8 {
                let nx = (x as isize + dx) as usize;
                let ny = (y as isize + dy) as usize;
                let b = *state.biome.get(nx, ny);
                counts[usize::from(b.as_u8())] += 1;
            }

            let mut best = center;
            let mut best_count = 0u8;
            for (idx, c) in counts.iter().enumerate() {
                if *c > best_count {
                    best_count = *c;
                    best = match idx as u8 {
                        0 => Biome::Ocean,
                        1 => Biome::Lake,
                        2 => Biome::PolarDesert,
                        3 => Biome::Tundra,
                        4 => Biome::BorealForest,
                        5 => Biome::TemperateGrassland,
                        6 => Biome::TemperateForest,
                        7 => Biome::Mediterranean,
                        8 => Biome::Savanna,
                        9 => Biome::TropicalSeasonalForest,
                        10 => Biome::TropicalRainforest,
                        11 => Biome::HotDesert,
                        12 => Biome::Alpine,
                        _ => Biome::Wetland,
                    };
                }
            }
            *out.get_mut(x, y) = best;
        }
    }

    state.biome = out;
}
