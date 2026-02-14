use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use image::{ImageBuffer, Rgb};
use serde::Serialize;

use crate::grid::Grid2D;
use crate::state::{Biome, RiverClass, Step, WorldState};

#[derive(Serialize)]
struct ExportMeta<'a> {
    seed: u64,
    width: usize,
    height: usize,
    step_state: Option<Step>,
    timings_ms: &'a BTreeMap<Step, f64>,
    checksum: &'a str,
    layer_hashes: &'a BTreeMap<String, String>,
    timestamp_unix_s: u64,
}

pub fn export_snapshot(state: &WorldState, dir: impl AsRef<Path>) -> Result<()> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir).with_context(|| format!("creating export dir {}", dir.display()))?;

    write_float_layer_png(&state.elevation, dir.join("elevation.png"))?;
    write_float_layer_png(&state.temperature, dir.join("temperature.png"))?;
    write_float_layer_png(&state.rainfall, dir.join("rainfall.png"))?;
    write_float_layer_png(&state.accumulation, dir.join("accumulation.png"))?;
    write_river_png(state, dir.join("river_class.png"))?;
    write_ocean_lake_png(state, dir.join("water_masks.png"))?;
    write_biome_png(&state.biome, dir.join("biome.png"))?;
    write_final_map_png(state, dir.join("final_map.png"))?;

    let meta = ExportMeta {
        seed: state.params.seed,
        width: state.width,
        height: state.height,
        step_state: state.current_step,
        timings_ms: &state.step_timings_ms,
        checksum: &state.diagnostics.checksum,
        layer_hashes: &state.diagnostics.layer_hashes,
        timestamp_unix_s: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    let meta_json = serde_json::to_string_pretty(&meta)?;
    fs::write(dir.join("meta.json"), meta_json)?;
    Ok(())
}

fn write_float_layer_png(grid: &Grid2D<f32>, path: impl AsRef<Path>) -> Result<()> {
    let min = grid
        .as_slice()
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let max = grid
        .as_slice()
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let span = (max - min).max(1e-9);

    let mut img = ImageBuffer::new(grid.width() as u32, grid.height() as u32);
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let v = (*grid.get(x, y) - min) / span;
            let c = (v.clamp(0.0, 1.0) * 255.0) as u8;
            img.put_pixel(x as u32, y as u32, Rgb([c, c, c]));
        }
    }
    img.save(path)?;
    Ok(())
}

fn write_river_png(state: &WorldState, path: impl AsRef<Path>) -> Result<()> {
    let mut img = ImageBuffer::new(state.width as u32, state.height as u32);
    for y in 0..state.height {
        for x in 0..state.width {
            let c: [u8; 3] = match *state.river_class.get(x, y) {
                RiverClass::None => [0, 0, 0],
                RiverClass::Ephemeral => [90, 170, 255],
                RiverClass::Perennial => [40, 130, 245],
                RiverClass::Major => [0, 60, 220],
            };
            img.put_pixel(x as u32, y as u32, Rgb(c));
        }
    }
    img.save(path)?;
    Ok(())
}

fn write_ocean_lake_png(state: &WorldState, path: impl AsRef<Path>) -> Result<()> {
    let mut img = ImageBuffer::new(state.width as u32, state.height as u32);
    for y in 0..state.height {
        for x in 0..state.width {
            let c: [u8; 3] = if *state.ocean_mask.get(x, y) {
                [0, 40, 120]
            } else if *state.lake_id.get(x, y) > 0 {
                [50, 120, 220]
            } else {
                [20, 20, 20]
            };
            img.put_pixel(x as u32, y as u32, Rgb(c));
        }
    }
    img.save(path)?;
    Ok(())
}

fn write_biome_png(grid: &Grid2D<Biome>, path: impl AsRef<Path>) -> Result<()> {
    let mut img = ImageBuffer::new(grid.width() as u32, grid.height() as u32);
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let c = biome_color(*grid.get(x, y));
            img.put_pixel(x as u32, y as u32, Rgb(c));
        }
    }
    img.save(path)?;
    Ok(())
}

fn write_final_map_png(state: &WorldState, path: impl AsRef<Path>) -> Result<()> {
    let mut img = ImageBuffer::new(state.width as u32, state.height as u32);

    for y in 0..state.height {
        for x in 0..state.width {
            let mut c = biome_color(*state.biome.get(x, y));

            let grad_x = if x > 0 && x + 1 < state.width {
                *state.elevation.get(x + 1, y) - *state.elevation.get(x - 1, y)
            } else {
                0.0
            };
            let grad_y = if y > 0 && y + 1 < state.height {
                *state.elevation.get(x, y + 1) - *state.elevation.get(x, y - 1)
            } else {
                0.0
            };
            let shade = (0.5 - grad_x * 2.4 - grad_y * 1.8).clamp(0.2, 0.9);
            c = scale_rgb(c, shade);

            if *state.ocean_mask.get(x, y) {
                c = [12, 44, 118];
            } else if *state.lake_id.get(x, y) > 0 {
                c = [44, 108, 196];
            }

            c = match *state.river_class.get(x, y) {
                RiverClass::None => c,
                RiverClass::Ephemeral => blend_rgb(c, [92, 170, 252], 0.18),
                RiverClass::Perennial => blend_rgb(c, [40, 120, 240], 0.7),
                RiverClass::Major => blend_rgb(c, [4, 74, 222], 0.85),
            };

            img.put_pixel(x as u32, y as u32, Rgb(c));
        }
    }

    img.save(path)?;
    Ok(())
}

fn biome_color(biome: Biome) -> [u8; 3] {
    match biome {
        Biome::Ocean => [18, 52, 120],
        Biome::Lake => [42, 98, 190],
        Biome::PolarDesert => [230, 230, 218],
        Biome::Tundra => [176, 190, 156],
        Biome::BorealForest => [66, 102, 63],
        Biome::TemperateGrassland => [158, 186, 102],
        Biome::TemperateForest => [74, 136, 80],
        Biome::Mediterranean => [146, 152, 84],
        Biome::Savanna => [186, 172, 78],
        Biome::TropicalSeasonalForest => [88, 144, 70],
        Biome::TropicalRainforest => [42, 116, 52],
        Biome::HotDesert => [214, 188, 126],
        Biome::Alpine => [136, 136, 140],
        Biome::Wetland => [76, 138, 120],
    }
}

fn scale_rgb(c: [u8; 3], k: f32) -> [u8; 3] {
    [
        (f32::from(c[0]) * k).clamp(0.0, 255.0) as u8,
        (f32::from(c[1]) * k).clamp(0.0, 255.0) as u8,
        (f32::from(c[2]) * k).clamp(0.0, 255.0) as u8,
    ]
}

fn blend_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let s = 1.0 - t;
    [
        (f32::from(a[0]) * s + f32::from(b[0]) * t).clamp(0.0, 255.0) as u8,
        (f32::from(a[1]) * s + f32::from(b[1]) * t).clamp(0.0, 255.0) as u8,
        (f32::from(a[2]) * s + f32::from(b[2]) * t).clamp(0.0, 255.0) as u8,
    ]
}
