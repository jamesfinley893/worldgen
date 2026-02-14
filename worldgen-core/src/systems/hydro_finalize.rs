use std::collections::VecDeque;

use crate::params::GenerationParams;
use crate::state::{RiverClass, WorldState};

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
    mark_ocean_component(state, params.base.sea_level);
    state.river_class.fill(RiverClass::None);
    state.lake_id.fill(0);

    for y in 0..state.height {
        for x in 0..state.width {
            if *state.ocean_mask.get(x, y) {
                continue;
            }
            let q = *state.discharge.get(x, y);
            let class = if q >= params.hydro.major_threshold {
                RiverClass::Major
            } else if q >= params.hydro.perennial_threshold {
                RiverClass::Perennial
            } else if q >= params.hydro.ephemeral_threshold {
                RiverClass::Ephemeral
            } else {
                RiverClass::None
            };
            *state.river_class.get_mut(x, y) = class;
        }
    }

    identify_lakes(state, params.base.sea_level);
}

fn mark_ocean_component(state: &mut WorldState, sea_level: f32) {
    state.ocean_mask.fill(false);
    let mut q = VecDeque::new();

    for x in 0..state.width {
        if *state.elevation.get(x, 0) <= sea_level {
            q.push_back((x, 0));
        }
        if *state.elevation.get(x, state.height - 1) <= sea_level {
            q.push_back((x, state.height - 1));
        }
    }
    for y in 0..state.height {
        if *state.elevation.get(0, y) <= sea_level {
            q.push_back((0, y));
        }
        if *state.elevation.get(state.width - 1, y) <= sea_level {
            q.push_back((state.width - 1, y));
        }
    }

    while let Some((x, y)) = q.pop_front() {
        if *state.ocean_mask.get(x, y) {
            continue;
        }
        if *state.elevation.get(x, y) > sea_level {
            continue;
        }
        *state.ocean_mask.get_mut(x, y) = true;

        for (dx, dy) in DIRS_8 {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if state.elevation.in_bounds(nx, ny) {
                q.push_back((nx as usize, ny as usize));
            }
        }
    }
}

fn identify_lakes(state: &mut WorldState, sea_level: f32) {
    let mut next_lake_id: u32 = 1;

    for y in 1..state.height.saturating_sub(1) {
        for x in 1..state.width.saturating_sub(1) {
            if *state.ocean_mask.get(x, y) || *state.lake_id.get(x, y) != 0 {
                continue;
            }
            let elev = *state.elevation.get(x, y);
            if elev <= sea_level {
                continue;
            }

            let is_local_pit = is_strict_local_pit(state, x, y, elev);
            if !is_local_pit {
                continue;
            }

            // Keep lakes conservative: only pit basins with enough inflow.
            if *state.accumulation.get(x, y) < 8.0 {
                continue;
            }

            let relief = local_relief(state, x, y);
            let max_level = elev + (0.001 + relief * 0.3).min(0.008);
            if flood_lake_basin(state, x, y, next_lake_id, max_level, 6_000) {
                next_lake_id = next_lake_id.saturating_add(1);
            }
        }
    }
}

fn is_strict_local_pit(state: &WorldState, x: usize, y: usize, elev: f32) -> bool {
    let eps = 1e-4;
    for (dx, dy) in DIRS_8 {
        let nx = (x as isize + dx) as usize;
        let ny = (y as isize + dy) as usize;
        let nh = *state.elevation.get(nx, ny);
        if nh <= elev + eps {
            return false;
        }
    }
    true
}

fn flood_lake_basin(
    state: &mut WorldState,
    sx: usize,
    sy: usize,
    lake_id: u32,
    max_level: f32,
    max_cells: usize,
) -> bool {
    let mut q = VecDeque::new();
    let mut cells: Vec<(usize, usize)> = Vec::new();
    let mut touches_edge = false;
    q.push_back((sx, sy));

    while let Some((x, y)) = q.pop_front() {
        if *state.ocean_mask.get(x, y) || *state.lake_id.get(x, y) == lake_id {
            continue;
        }
        if *state.elevation.get(x, y) > max_level {
            continue;
        }
        if *state.lake_id.get(x, y) != 0 {
            touches_edge = true;
            continue;
        }
        if x == 0 || y == 0 || x + 1 == state.width || y + 1 == state.height {
            touches_edge = true;
        }
        *state.lake_id.get_mut(x, y) = lake_id;
        cells.push((x, y));
        if cells.len() > max_cells {
            touches_edge = true;
            break;
        }

        for (dx, dy) in DIRS_8 {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if state.elevation.in_bounds(nx, ny) {
                q.push_back((nx as usize, ny as usize));
            }
        }
    }

    if touches_edge {
        for (x, y) in cells {
            *state.lake_id.get_mut(x, y) = 0;
        }
        return false;
    }

    true
}

fn local_relief(state: &WorldState, x: usize, y: usize) -> f32 {
    let mut min_h = *state.elevation.get(x, y);
    let mut max_h = min_h;
    for (dx, dy) in DIRS_8 {
        let nx = (x as isize + dx) as usize;
        let ny = (y as isize + dy) as usize;
        let h = *state.elevation.get(nx, ny);
        min_h = min_h.min(h);
        max_h = max_h.max(h);
    }
    max_h - min_h
}
