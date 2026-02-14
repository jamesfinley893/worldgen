# DF-Style Offline Worldgen (Rust)

This workspace implements deterministic tile-based world generation for steps 1-5:

1. Base fields (elevation, temperature, rainfall, pressure/wind proxies)
2. Erosion + hydrology passes (sink handling, D8 routing, accumulation/discharge, hydraulic + thermal smoothing)
3. Biome classification (deterministic thresholds + smoothing + fertility proxy)
4. Rivers/lakes/oceans finalization
5. Geology/strata + minerals

## Workspace Layout

- `worldgen-core`: generation library + scheduler + export (`PNG` + `meta.json`)
- `worldgen-ui`: `eframe/egui` desktop controller/viewer

## Build / Run

```bash
cargo fmt
cargo check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p worldgen-core
cargo run -p worldgen-ui
```

## Windows Cross-Compile

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu -p worldgen-ui
```

## Determinism

`worldgen-core` includes regression tests:

- same seed + params => same checksum
- different seed => different checksum

Checksums are computed from deterministic per-layer hashes (BLAKE3).

## Export

In UI, click `Export Snapshot`.

Exports are written to:

- `exports/seed<seed>_<checksum-prefix>/`

Output includes:

- PNG layers (`elevation`, `temperature`, `rainfall`, `accumulation`, `river_class`, `water_masks`, `biome`)
- `meta.json` with seed, size, step state, timings, hashes, checksum, timestamp
# worldgen
