use crate::params::GenerationParams;
use crate::rng::hash_2d;
use crate::state::WorldState;

pub fn run(state: &mut WorldState, params: &GenerationParams) {
    build_elevation_and_atmosphere(state, params);
    smooth_elevation(state, 2);
    rebalance_elevation_distribution(state, params.base.sea_level);
    build_temperature(state, params);
    simulate_moisture_transport(state, params);
}

fn build_elevation_and_atmosphere(state: &mut WorldState, params: &GenerationParams) {
    let width = state.width as f32;
    let height = state.height as f32;
    let octaves = params.base.octaves.max(1);

    for y in 0..state.height {
        let lat = (y as f32 + 0.5) / height;

        let hadley = (lat * std::f32::consts::TAU * 3.0).sin();
        let pressure_band = (0.5 + 0.45 * hadley).clamp(0.0, 1.0);
        let zonal = (lat * std::f32::consts::TAU * 2.0).sin().clamp(-1.0, 1.0);
        let meridional = (0.5 - lat).signum() * (1.0 - zonal.abs() * 0.65);

        for x in 0..state.width {
            let nx = x as f32 / width;
            let ny = y as f32 / height;

            let warp_x = fbm(
                seed_offset(params.seed, 101),
                nx * 0.9,
                ny * 0.9,
                3,
                params.base.frequency * 0.65,
            );
            let warp_y = fbm(
                seed_offset(params.seed, 202),
                nx * 0.9,
                ny * 0.9,
                3,
                params.base.frequency * 0.65,
            );

            let wx = nx + (warp_x - 0.5) * params.base.warp_strength * 1.8;
            let wy = ny + (warp_y - 0.5) * params.base.warp_strength * 1.8;

            let continental = fbm(
                seed_offset(params.seed, 303),
                wx,
                wy,
                octaves,
                params.base.frequency * 1.05,
            );
            let macro_plate = fbm(
                seed_offset(params.seed, 404),
                wx * 0.55,
                wy * 0.55,
                4,
                params.base.frequency * 0.8,
            );
            let ridges = ridged_fbm(
                seed_offset(params.seed, 505),
                wx,
                wy,
                5,
                params.base.frequency * 1.1,
            );
            let mountain_belts = ridged_fbm(
                seed_offset(params.seed, 606),
                wx * 0.6,
                wy * 0.6,
                4,
                params.base.frequency * 1.0,
            );
            let basin = fbm(
                seed_offset(params.seed, 707),
                wx * 1.4,
                wy * 1.4,
                4,
                params.base.frequency * 1.25,
            );

            let continentality =
                ((continental - 0.5) * 1.25 + (macro_plate - 0.5) * 0.7 + 0.5).clamp(0.0, 1.0);
            let uplift = ((0.55 * ridges + 0.45 * mountain_belts).powf(1.3)).clamp(0.0, 1.0);

            let mut elev = continentality * 0.76 + uplift * 0.24 - basin * 0.16;
            elev = (elev * elev).clamp(0.0, 1.0);

            *state.elevation.get_mut(x, y) = elev;
            *state.pressure.get_mut(x, y) = pressure_band;
            *state.wind_u.get_mut(x, y) = zonal;
            *state.wind_v.get_mut(x, y) = meridional;
        }
    }
}

fn build_temperature(state: &mut WorldState, params: &GenerationParams) {
    let height = state.height as f32;
    for y in 0..state.height {
        let lat = (y as f32 + 0.5) / height;
        let lat_factor = (lat - 0.5).abs() * 2.0;

        for x in 0..state.width {
            let elev = *state.elevation.get(x, y);
            let oceanic = if elev <= params.base.sea_level {
                1.0
            } else {
                0.0
            };
            let elev_km = ((elev - params.base.sea_level).max(0.0)) * 7.5;
            let base_temp_c = 33.0 - 57.0 * lat_factor;
            let maritime = oceanic * (1.0 - lat_factor) * 2.5;
            let temp = base_temp_c + maritime - params.base.lapse_rate_c_per_km * elev_km;
            *state.temperature.get_mut(x, y) = temp;
        }
    }
}

fn simulate_moisture_transport(state: &mut WorldState, params: &GenerationParams) {
    let w = state.width;
    let h = state.height;
    let mut moisture = vec![0.0f32; w * h];
    let mut next = vec![0.0f32; w * h];
    state.rainfall.fill(0.0);

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let ocean = *state.elevation.get(x, y) <= params.base.sea_level;
            moisture[idx] = if ocean { 0.9 } else { 0.12 };
        }
    }

    for _ in 0..42 {
        next.fill(0.0);

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let elev = *state.elevation.get(x, y);
                let ocean = elev <= params.base.sea_level;
                let mut m = moisture[idx];

                if ocean {
                    m += 0.10;
                }

                let wx = *state.wind_u.get(x, y);
                let wy = *state.wind_v.get(x, y);
                let wind_mag = (wx * wx + wy * wy).sqrt().clamp(0.0, 1.2);

                let upwind_x = if wx > 0.0 {
                    x.saturating_sub(1)
                } else {
                    (x + 1).min(w - 1)
                };
                let upwind_y = if wy > 0.0 {
                    y.saturating_sub(1)
                } else {
                    (y + 1).min(h - 1)
                };
                let upwind_elev = *state.elevation.get(upwind_x, upwind_y);
                let uplift = (elev - upwind_elev).max(0.0);

                let convective = ((*state.temperature.get(x, y) + 8.0) / 44.0).clamp(0.0, 1.0);
                let precip_rate =
                    (0.015 + uplift * 1.25 + convective * 0.06 + wind_mag * 0.02).clamp(0.01, 0.85);
                let precip = (m * precip_rate).min(m);
                *state.rainfall.get_mut(x, y) += precip;
                m -= precip;

                let evap = if ocean { 0.06 } else { 0.0 };
                m += evap;
                m *= 0.992;

                let advect = (0.12 + 0.6 * wind_mag).clamp(0.08, 0.8);
                let transfer = m * advect;
                let retain = m - transfer;
                next[idx] += retain;

                let dx = wx.signum() as isize;
                let dy = wy.signum() as isize;
                let nx = (x as isize + dx).clamp(0, (w - 1) as isize) as usize;
                let ny = (y as isize + dy).clamp(0, (h - 1) as isize) as usize;
                let nidx = ny * w + nx;
                next[nidx] += transfer;
            }
        }

        std::mem::swap(&mut moisture, &mut next);
    }

    let mut max_rain = 0.0f32;
    for v in state.rainfall.as_slice() {
        max_rain = max_rain.max(*v);
    }
    let inv = if max_rain > 0.0 { 1.0 / max_rain } else { 1.0 };
    for v in state.rainfall.as_mut_slice() {
        *v = (*v * inv).clamp(0.0, 1.0);
    }
}

fn smooth_elevation(state: &mut WorldState, passes: usize) {
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

    for _ in 0..passes {
        let mut out = state.elevation.clone();
        for y in 1..state.height.saturating_sub(1) {
            for x in 1..state.width.saturating_sub(1) {
                let mut sum = *state.elevation.get(x, y) * 0.55;
                let mut weight = 0.55;
                for (dx, dy) in DIRS_8 {
                    let nx = (x as isize + dx) as usize;
                    let ny = (y as isize + dy) as usize;
                    let w = if dx == 0 || dy == 0 { 0.08 } else { 0.045 };
                    sum += *state.elevation.get(nx, ny) * w;
                    weight += w;
                }
                *out.get_mut(x, y) = (sum / weight).clamp(0.0, 1.0);
            }
        }
        state.elevation = out;
    }
}

fn rebalance_elevation_distribution(state: &mut WorldState, sea_level: f32) {
    let mut values = state.elevation.as_slice().to_vec();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len().saturating_sub(1);
    let t = sea_level.clamp(0.05, 0.95);
    let kth = (t * n as f32) as usize;
    let level_at_quantile = values[kth];
    let shift = sea_level - level_at_quantile;

    for v in state.elevation.as_mut_slice() {
        *v = (*v + shift).clamp(0.0, 1.0);
    }

    for v in state.elevation.as_mut_slice() {
        let d = *v - sea_level;
        *v = (sea_level + d * 1.2).clamp(0.0, 1.0);
    }
}

fn seed_offset(seed: u64, offset: u64) -> u64 {
    seed ^ (offset.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let tx = x - xi as f32;
    let ty = y - yi as f32;

    let v00 = hash_2d(seed, xi, yi);
    let v10 = hash_2d(seed, xi + 1, yi);
    let v01 = hash_2d(seed, xi, yi + 1);
    let v11 = hash_2d(seed, xi + 1, yi + 1);

    let sx = smoothstep(tx);
    let sy = smoothstep(ty);

    let a = lerp(v00, v10, sx);
    let b = lerp(v01, v11, sx);
    lerp(a, b, sy)
}

fn fbm(seed: u64, x: f32, y: f32, octaves: u32, base_freq: f32) -> f32 {
    let mut amp = 0.5;
    let mut freq = base_freq;
    let mut sum = 0.0;
    let mut norm = 0.0;

    for octave in 0..octaves {
        let n = value_noise(seed_offset(seed, octave as u64 + 1), x * freq, y * freq);
        sum += n * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }

    if norm > 0.0 {
        sum / norm
    } else {
        0.0
    }
}

fn ridged_fbm(seed: u64, x: f32, y: f32, octaves: u32, base_freq: f32) -> f32 {
    let mut amp = 0.5;
    let mut freq = base_freq;
    let mut sum = 0.0;
    let mut norm = 0.0;

    for octave in 0..octaves {
        let n = value_noise(seed_offset(seed, octave as u64 + 1), x * freq, y * freq);
        let ridge = 1.0 - (2.0 * n - 1.0).abs();
        sum += ridge * ridge * amp;
        norm += amp;
        amp *= 0.55;
        freq *= 2.0;
    }

    if norm > 0.0 {
        (sum / norm).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
