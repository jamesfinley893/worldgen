use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapSizePreset {
    S256,
    S512,
    S1024,
}

impl MapSizePreset {
    pub fn dimensions(self) -> (usize, usize) {
        match self {
            Self::S256 => (256, 256),
            Self::S512 => (512, 512),
            Self::S1024 => (1024, 1024),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseFieldParams {
    pub sea_level: f32,
    pub octaves: u32,
    pub frequency: f32,
    pub warp_strength: f32,
    pub lapse_rate_c_per_km: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErosionParams {
    pub iterations: u32,
    pub erosion_rate: f32,
    pub deposition_rate: f32,
    pub thermal_rate: f32,
    pub min_slope: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BiomeParams {
    pub smoothing_passes: u32,
    pub wetness_weight: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HydroFinalizeParams {
    pub ephemeral_threshold: f32,
    pub perennial_threshold: f32,
    pub major_threshold: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeologyParams {
    pub strata_layers: u8,
    pub fault_strength: f32,
    pub ore_richness: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationParams {
    pub seed: u64,
    pub size: MapSizePreset,
    pub base: BaseFieldParams,
    pub erosion: ErosionParams,
    pub biomes: BiomeParams,
    pub hydro: HydroFinalizeParams,
    pub geology: GeologyParams,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            seed: 42,
            size: MapSizePreset::S512,
            base: BaseFieldParams {
                sea_level: 0.5,
                octaves: 6,
                frequency: 2.1,
                warp_strength: 0.03,
                lapse_rate_c_per_km: 6.5,
            },
            erosion: ErosionParams {
                iterations: 24,
                erosion_rate: 0.035,
                deposition_rate: 0.02,
                thermal_rate: 0.015,
                min_slope: 0.0008,
            },
            biomes: BiomeParams {
                smoothing_passes: 2,
                wetness_weight: 0.4,
            },
            hydro: HydroFinalizeParams {
                ephemeral_threshold: 80.0,
                perennial_threshold: 260.0,
                major_threshold: 900.0,
            },
            geology: GeologyParams {
                strata_layers: 6,
                fault_strength: 0.5,
                ore_richness: 0.35,
            },
        }
    }
}
