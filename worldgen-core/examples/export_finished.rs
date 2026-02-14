use anyhow::Result;
use worldgen_core::export::export_snapshot;
use worldgen_core::{run_all_steps, GenerationParams, MapSizePreset, WorldState};

fn main() -> Result<()> {
    let mut size = MapSizePreset::S1024;
    let mut seed: Option<u64> = None;
    for arg in std::env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--seed=") {
            if value == "random" {
                seed = Some(random_seed());
            } else if let Ok(parsed) = value.parse::<u64>() {
                seed = Some(parsed);
            }
            continue;
        }

        match arg.as_str() {
            "--size=256" => size = MapSizePreset::S256,
            "--size=512" => size = MapSizePreset::S512,
            "--size=1024" => size = MapSizePreset::S1024,
            _ => {}
        }
    }

    let params = GenerationParams {
        seed: seed.unwrap_or(42),
        size,
        ..GenerationParams::default()
    };

    let mut state = WorldState::new(params.clone());
    run_all_steps(&mut state, &params)?;

    let out_dir = format!(
        "exports/finished_seed{}_{}x{}",
        params.seed, state.width, state.height
    );
    export_snapshot(&state, &out_dir)?;

    println!("exported: {out_dir}");
    println!("checksum: {}", state.diagnostics.checksum);
    Ok(())
}

fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| (d.as_nanos() as u64) ^ d.as_secs().rotate_left(21))
}
