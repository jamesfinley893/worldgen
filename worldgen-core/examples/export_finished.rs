use anyhow::Result;
use worldgen_core::export::export_snapshot;
use worldgen_core::{run_all_steps, GenerationParams, MapSizePreset, WorldState};

fn main() -> Result<()> {
    let mut size = MapSizePreset::S1024;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--size=256" => size = MapSizePreset::S256,
            "--size=512" => size = MapSizePreset::S512,
            "--size=1024" => size = MapSizePreset::S1024,
            _ => {}
        }
    }

    let params = GenerationParams {
        seed: 42,
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
