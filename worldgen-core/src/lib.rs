pub mod export;
pub mod grid;
pub mod params;
pub mod rng;
pub mod scheduler;
pub mod state;
pub mod systems;
pub mod time;

pub use params::{GenerationParams, MapSizePreset};
pub use scheduler::{run_all_steps, run_next_step, run_step};
pub use state::{Step, WorldState};

#[cfg(test)]
mod tests {
    use crate::{run_all_steps, GenerationParams, MapSizePreset, WorldState};

    #[test]
    fn deterministic_same_seed_same_checksum() {
        let p = GenerationParams {
            seed: 123_456_789,
            size: MapSizePreset::S256,
            ..GenerationParams::default()
        };

        let mut a = WorldState::new(p.clone());
        run_all_steps(&mut a, &p).expect("run a");
        let hash_a = a.diagnostics.checksum.clone();

        let mut b = WorldState::new(p.clone());
        run_all_steps(&mut b, &p).expect("run b");
        let hash_b = b.diagnostics.checksum.clone();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn deterministic_different_seed_different_checksum() {
        let p1 = GenerationParams {
            seed: 111,
            size: MapSizePreset::S256,
            ..GenerationParams::default()
        };
        let mut a = WorldState::new(p1.clone());
        run_all_steps(&mut a, &p1).expect("run a");

        let mut p2 = p1;
        p2.seed = 222;
        let mut b = WorldState::new(p2.clone());
        run_all_steps(&mut b, &p2).expect("run b");

        assert_ne!(a.diagnostics.checksum, b.diagnostics.checksum);
    }
}
