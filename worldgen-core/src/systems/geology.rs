use crate::params::GenerationParams;
use crate::rng::hash_2d;
use crate::state::{GeologicProvince, Mineral, RockType, StrataLayer, WorldState};

pub fn run(state: &mut WorldState, params: &GenerationParams) {
    assign_provinces(state, params);
    assign_strata_and_rock(state, params);
    assign_minerals(state, params);
}

fn assign_provinces(state: &mut WorldState, params: &GenerationParams) {
    for y in 0..state.height {
        for x in 0..state.width {
            let elev = *state.elevation.get(x, y);
            let slope = local_slope(state, x, y);
            let noise = hash_2d(params.seed ^ 0xABCDEF01, x as i32, y as i32);

            let province = if *state.ocean_mask.get(x, y) {
                GeologicProvince::Oceanic
            } else if slope > 0.03 && elev > params.base.sea_level + 0.2 {
                if noise > 0.72 {
                    GeologicProvince::VolcanicArc
                } else {
                    GeologicProvince::Orogen
                }
            } else if elev < params.base.sea_level + 0.07 {
                GeologicProvince::Basin
            } else {
                GeologicProvince::Craton
            };

            *state.geologic_province.get_mut(x, y) = province;
        }
    }
}

fn assign_strata_and_rock(state: &mut WorldState, params: &GenerationParams) {
    let layers = params.geology.strata_layers.max(3);
    for y in 0..state.height {
        for x in 0..state.width {
            let province = *state.geologic_province.get(x, y);
            let base_rock = match province {
                GeologicProvince::Oceanic => RockType::Basalt,
                GeologicProvince::Craton => RockType::Granite,
                GeologicProvince::Orogen => RockType::Schist,
                GeologicProvince::Basin => RockType::Shale,
                GeologicProvince::VolcanicArc => RockType::Rhyolite,
            };
            *state.rock_type.get_mut(x, y) = base_rock;

            let mut stack = Vec::with_capacity(usize::from(layers));
            for l in 0..layers {
                let n = hash_2d(params.seed ^ 0x55115511 ^ u64::from(l), x as i32, y as i32);
                let thickness = 8.0 + n * 28.0;
                let rock = layer_rock(province, l, n);
                stack.push(StrataLayer { rock, thickness });
            }

            if params.geology.fault_strength > 0.0 {
                let fault = hash_2d(params.seed ^ 0xDEADBEEF, x as i32, y as i32);
                if fault > 1.0 - params.geology.fault_strength * 0.12 && stack.len() > 1 {
                    stack.swap(0, 1);
                }
            }

            *state.strata.get_mut(x, y) = stack;
        }
    }
}

fn assign_minerals(state: &mut WorldState, params: &GenerationParams) {
    for mask in state.mineral_masks.values_mut() {
        mask.fill(false);
    }

    for y in 0..state.height {
        for x in 0..state.width {
            let province = *state.geologic_province.get(x, y);
            let rock = *state.rock_type.get(x, y);

            for mineral in Mineral::ALL {
                let key = format!("{:?}", mineral).to_lowercase();
                let score = mineral_score(params.seed, x, y, mineral, rock, province);
                let threshold = match mineral {
                    Mineral::Iron => 0.72,
                    Mineral::Copper => 0.78,
                    Mineral::Gold => 0.9,
                    Mineral::Tin => 0.82,
                    Mineral::Coal => 0.74,
                    Mineral::Gem => 0.94,
                } - params.geology.ore_richness * 0.25;

                if score > threshold {
                    if let Some(mask) = state.mineral_masks.get_mut(&key) {
                        *mask.get_mut(x, y) = true;
                    }
                }
            }
        }
    }
}

fn layer_rock(province: GeologicProvince, layer: u8, n: f32) -> RockType {
    match province {
        GeologicProvince::Oceanic => {
            if layer == 0 || n > 0.6 {
                RockType::Basalt
            } else {
                RockType::Gabbro
            }
        }
        GeologicProvince::Craton => {
            if layer.is_multiple_of(3) {
                RockType::Granite
            } else if n > 0.66 {
                RockType::Gneiss
            } else {
                RockType::Sandstone
            }
        }
        GeologicProvince::Orogen => {
            if n > 0.52 {
                RockType::Schist
            } else {
                RockType::Gneiss
            }
        }
        GeologicProvince::Basin => {
            if n > 0.6 {
                RockType::Limestone
            } else if n > 0.25 {
                RockType::Shale
            } else {
                RockType::Sandstone
            }
        }
        GeologicProvince::VolcanicArc => {
            if n > 0.55 {
                RockType::Rhyolite
            } else {
                RockType::Basalt
            }
        }
    }
}

fn mineral_score(
    seed: u64,
    x: usize,
    y: usize,
    mineral: Mineral,
    rock: RockType,
    province: GeologicProvince,
) -> f32 {
    let n = hash_2d(
        seed ^ (u64::from(mineral.as_u8()) << 32),
        x as i32,
        y as i32,
    );
    let host_bonus = match (mineral, rock) {
        (Mineral::Iron, RockType::Basalt | RockType::Gabbro) => 0.16,
        (Mineral::Copper, RockType::Rhyolite | RockType::Basalt) => 0.18,
        (Mineral::Gold, RockType::Schist | RockType::Gneiss) => 0.17,
        (Mineral::Tin, RockType::Granite) => 0.14,
        (Mineral::Coal, RockType::Shale | RockType::Sandstone) => 0.2,
        (Mineral::Gem, RockType::Schist | RockType::Gneiss | RockType::Rhyolite) => 0.12,
        _ => 0.0,
    };

    let province_bonus = match (mineral, province) {
        (Mineral::Iron, GeologicProvince::Oceanic | GeologicProvince::VolcanicArc) => 0.12,
        (Mineral::Copper, GeologicProvince::VolcanicArc) => 0.13,
        (Mineral::Gold, GeologicProvince::Orogen) => 0.14,
        (Mineral::Tin, GeologicProvince::Craton) => 0.1,
        (Mineral::Coal, GeologicProvince::Basin) => 0.16,
        (Mineral::Gem, GeologicProvince::Orogen) => 0.1,
        _ => 0.0,
    };

    (n + host_bonus + province_bonus).clamp(0.0, 1.0)
}

fn local_slope(state: &WorldState, x: usize, y: usize) -> f32 {
    let h = *state.elevation.get(x, y);
    let mut max_diff: f32 = 0.0;

    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if state.elevation.in_bounds(nx, ny) {
                let nh = *state.elevation.get(nx as usize, ny as usize);
                max_diff = max_diff.max((h - nh).abs());
            }
        }
    }

    max_diff
}
