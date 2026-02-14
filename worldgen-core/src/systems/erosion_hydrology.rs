use std::cmp::Ordering;

use crate::params::GenerationParams;
use crate::rng::hash_2d;
use crate::state::WorldState;

const DIRS: [(isize, isize); 8] = [
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
    fill_depressions(state, params.base.sea_level);

    for _ in 0..params.erosion.iterations {
        compute_flow_d8(state);
        compute_accumulation(state);
        apply_hydraulic_erosion(state, params);
        apply_thermal_relaxation(state, params);
    }

    compute_flow_d8(state);
    compute_accumulation(state);
}

fn fill_depressions(state: &mut WorldState, sea_level: f32) {
    let epsilon = 1e-5;
    for _ in 0..8 {
        let mut changed = false;
        for y in 1..state.height.saturating_sub(1) {
            for x in 1..state.width.saturating_sub(1) {
                let cur = *state.elevation.get(x, y);
                if cur <= sea_level {
                    continue;
                }

                let mut min_nb = f32::MAX;
                for (dx, dy) in DIRS {
                    let nx = (x as isize + dx) as usize;
                    let ny = (y as isize + dy) as usize;
                    min_nb = min_nb.min(*state.elevation.get(nx, ny));
                }

                if cur < min_nb {
                    *state.elevation.get_mut(x, y) = min_nb + epsilon;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn compute_flow_d8(state: &mut WorldState) {
    state.flow_dir.fill(255);
    for y in 0..state.height {
        for x in 0..state.width {
            let h = *state.elevation.get(x, y);
            let mut best_metric = 0.0f32;
            let mut best_dir = 255u8;
            let mut best_tie = 0.0f32;

            for (i, (dx, dy)) in DIRS.iter().enumerate() {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if !state.elevation.in_bounds(nx, ny) {
                    continue;
                }
                let nh = *state.elevation.get(nx as usize, ny as usize);
                let drop = h - nh;
                if drop <= 0.0 {
                    continue;
                }
                let dist = if *dx != 0 && *dy != 0 {
                    std::f32::consts::SQRT_2
                } else {
                    1.0
                };
                let metric = drop / dist;
                let tie = hash_2d(
                    state.params.seed ^ 0xBADC0FFE,
                    nx as i32 + i as i32,
                    ny as i32 - i as i32,
                );
                if metric > best_metric + 1e-8
                    || ((metric - best_metric).abs() <= 1e-8 && tie > best_tie)
                {
                    best_metric = metric;
                    best_tie = tie;
                    best_dir = i as u8;
                }
            }
            *state.flow_dir.get_mut(x, y) = best_dir;
        }
    }
}

fn compute_accumulation(state: &mut WorldState) {
    state.accumulation.fill(1.0);

    let mut order: Vec<(usize, usize, f32)> = Vec::with_capacity(state.width * state.height);
    for (x, y) in state.elevation.iter_coords() {
        order.push((x, y, *state.elevation.get(x, y)));
    }
    order.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));

    for (x, y, _) in order {
        let h = *state.elevation.get(x, y);
        let q = *state.accumulation.get(x, y);

        let mut targets: [(usize, usize, f32); 8] = [(0, 0, 0.0); 8];
        let mut n_targets = 0usize;
        let mut wsum = 0.0f32;

        for (dx, dy) in DIRS {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if !state.elevation.in_bounds(nx, ny) {
                continue;
            }
            let nh = *state.elevation.get(nx as usize, ny as usize);
            let drop = h - nh;
            if drop <= 0.0 {
                continue;
            }
            let dist = if dx != 0 && dy != 0 {
                std::f32::consts::SQRT_2
            } else {
                1.0
            };
            let slope = drop / dist;
            let w = slope.powf(1.15);
            targets[n_targets] = (nx as usize, ny as usize, w);
            n_targets += 1;
            wsum += w;
        }

        if n_targets == 0 || wsum <= 0.0 {
            continue;
        }

        for &(nx, ny, w) in &targets[..n_targets] {
            let frac = w / wsum;
            let dst = state.accumulation.get_mut(nx, ny);
            *dst += q * frac;
        }
    }

    state.discharge.clone_from(&state.accumulation);
}

fn apply_hydraulic_erosion(state: &mut WorldState, params: &GenerationParams) {
    let width = state.width;
    let height = state.height;
    let mut delta = vec![0.0f32; width * height];

    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let idx = state.elevation.idx(x, y);
            let dir = *state.flow_dir.get(x, y);
            if dir == 255 {
                continue;
            }
            let h = *state.elevation.get(x, y);
            let (dx, dy) = DIRS[usize::from(dir)];
            let nx = (x as isize + dx) as usize;
            let ny = (y as isize + dy) as usize;
            let nh = *state.elevation.get(nx, ny);
            let slope = (h - nh).max(0.0);
            if slope < params.erosion.min_slope {
                continue;
            }

            let q = *state.discharge.get(x, y);
            let stream_power = q.sqrt() * slope;
            let capacity = stream_power * 0.08;
            let sediment = *state.rainfall.get(x, y) * 0.4;

            if sediment < capacity {
                let erode = (capacity - sediment) * params.erosion.erosion_rate;
                delta[idx] -= erode;
                let nidx = state.elevation.idx(nx, ny);
                delta[nidx] += erode * params.erosion.deposition_rate;
            } else {
                let deposit = (sediment - capacity) * params.erosion.deposition_rate;
                delta[idx] += deposit;
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            let idx = state.elevation.idx(x, y);
            let new_h = (*state.elevation.get(x, y) + delta[idx]).clamp(0.0, 1.0);
            *state.elevation.get_mut(x, y) = new_h;
        }
    }
}

fn apply_thermal_relaxation(state: &mut WorldState, params: &GenerationParams) {
    let width = state.width;
    let height = state.height;
    let mut out = state.elevation.clone();

    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let h = *state.elevation.get(x, y);
            let mut sum = 0.0;
            let mut count = 0.0;
            for (dx, dy) in DIRS {
                let nx = (x as isize + dx) as usize;
                let ny = (y as isize + dy) as usize;
                sum += *state.elevation.get(nx, ny);
                count += 1.0;
            }
            let avg = sum / count;
            let relaxed = h + (avg - h) * params.erosion.thermal_rate;
            *out.get_mut(x, y) = relaxed;
        }
    }

    state.elevation = out;
}
