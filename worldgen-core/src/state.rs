use std::collections::BTreeMap;

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::grid::Grid2D;
use crate::params::GenerationParams;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Step {
    BaseFields,
    ErosionHydrology,
    Biomes,
    HydroFinalize,
    Geology,
}

impl Step {
    pub const ALL: [Step; 5] = [
        Step::BaseFields,
        Step::ErosionHydrology,
        Step::Biomes,
        Step::HydroFinalize,
        Step::Geology,
    ];

    pub fn index(self) -> usize {
        match self {
            Step::BaseFields => 1,
            Step::ErosionHydrology => 2,
            Step::Biomes => 3,
            Step::HydroFinalize => 4,
            Step::Geology => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiverClass {
    None,
    Ephemeral,
    Perennial,
    Major,
}

impl RiverClass {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Ephemeral => 1,
            Self::Perennial => 2,
            Self::Major => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Biome {
    Ocean,
    Lake,
    PolarDesert,
    Tundra,
    BorealForest,
    TemperateGrassland,
    TemperateForest,
    Mediterranean,
    Savanna,
    TropicalSeasonalForest,
    TropicalRainforest,
    HotDesert,
    Alpine,
    Wetland,
}

impl Biome {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Ocean => 0,
            Self::Lake => 1,
            Self::PolarDesert => 2,
            Self::Tundra => 3,
            Self::BorealForest => 4,
            Self::TemperateGrassland => 5,
            Self::TemperateForest => 6,
            Self::Mediterranean => 7,
            Self::Savanna => 8,
            Self::TropicalSeasonalForest => 9,
            Self::TropicalRainforest => 10,
            Self::HotDesert => 11,
            Self::Alpine => 12,
            Self::Wetland => 13,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeologicProvince {
    Oceanic,
    Craton,
    Orogen,
    Basin,
    VolcanicArc,
}

impl GeologicProvince {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Oceanic => 0,
            Self::Craton => 1,
            Self::Orogen => 2,
            Self::Basin => 3,
            Self::VolcanicArc => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RockType {
    Basalt,
    Gabbro,
    Granite,
    Sandstone,
    Limestone,
    Schist,
    Gneiss,
    Shale,
    Rhyolite,
}

impl RockType {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Basalt => 0,
            Self::Gabbro => 1,
            Self::Granite => 2,
            Self::Sandstone => 3,
            Self::Limestone => 4,
            Self::Schist => 5,
            Self::Gneiss => 6,
            Self::Shale => 7,
            Self::Rhyolite => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Mineral {
    Iron,
    Copper,
    Gold,
    Tin,
    Coal,
    Gem,
}

impl Mineral {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Iron => 0,
            Self::Copper => 1,
            Self::Gold => 2,
            Self::Tin => 3,
            Self::Coal => 4,
            Self::Gem => 5,
        }
    }

    pub const ALL: [Mineral; 6] = [
        Mineral::Iron,
        Mineral::Copper,
        Mineral::Gold,
        Mineral::Tin,
        Mineral::Coal,
        Mineral::Gem,
    ];
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StrataLayer {
    pub rock: RockType,
    pub thickness: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostics {
    pub layer_hashes: BTreeMap<String, String>,
    pub checksum: String,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            layer_hashes: BTreeMap::new(),
            checksum: String::from("unset"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldState {
    pub width: usize,
    pub height: usize,

    pub elevation: Grid2D<f32>,
    pub temperature: Grid2D<f32>,
    pub rainfall: Grid2D<f32>,
    pub pressure: Grid2D<f32>,
    pub wind_u: Grid2D<f32>,
    pub wind_v: Grid2D<f32>,

    pub flow_dir: Grid2D<u8>,
    pub accumulation: Grid2D<f32>,
    pub discharge: Grid2D<f32>,
    pub river_class: Grid2D<RiverClass>,
    pub lake_id: Grid2D<u32>,
    pub ocean_mask: Grid2D<bool>,

    pub biome: Grid2D<Biome>,
    pub fertility: Grid2D<f32>,

    pub geologic_province: Grid2D<GeologicProvince>,
    pub strata: Grid2D<Vec<StrataLayer>>,
    pub rock_type: Grid2D<RockType>,
    pub mineral_masks: BTreeMap<String, Grid2D<bool>>,

    pub current_step: Option<Step>,
    pub step_timings_ms: BTreeMap<Step, f64>,
    pub params: GenerationParams,
    pub diagnostics: Diagnostics,
}

impl WorldState {
    pub fn new(params: GenerationParams) -> Self {
        let (width, height) = params.size.dimensions();
        let mut mineral_masks = BTreeMap::new();
        for mineral in Mineral::ALL {
            mineral_masks.insert(
                format!("{:?}", mineral).to_lowercase(),
                Grid2D::new(width, height, false),
            );
        }

        Self {
            width,
            height,
            elevation: Grid2D::new(width, height, 0.0),
            temperature: Grid2D::new(width, height, 0.0),
            rainfall: Grid2D::new(width, height, 0.0),
            pressure: Grid2D::new(width, height, 0.0),
            wind_u: Grid2D::new(width, height, 0.0),
            wind_v: Grid2D::new(width, height, 0.0),
            flow_dir: Grid2D::new(width, height, 255),
            accumulation: Grid2D::new(width, height, 0.0),
            discharge: Grid2D::new(width, height, 0.0),
            river_class: Grid2D::new(width, height, RiverClass::None),
            lake_id: Grid2D::new(width, height, 0),
            ocean_mask: Grid2D::new(width, height, false),
            biome: Grid2D::new(width, height, Biome::Ocean),
            fertility: Grid2D::new(width, height, 0.0),
            geologic_province: Grid2D::new(width, height, GeologicProvince::Craton),
            strata: Grid2D::new(width, height, Vec::new()),
            rock_type: Grid2D::new(width, height, RockType::Granite),
            mineral_masks,
            current_step: None,
            step_timings_ms: BTreeMap::new(),
            params,
            diagnostics: Diagnostics::default(),
        }
    }

    pub fn update_diagnostics(&mut self) {
        let mut hashes = BTreeMap::new();
        hashes.insert("elevation".to_string(), self.hash_f32(&self.elevation));
        hashes.insert("temperature".to_string(), self.hash_f32(&self.temperature));
        hashes.insert("rainfall".to_string(), self.hash_f32(&self.rainfall));
        hashes.insert(
            "accumulation".to_string(),
            self.hash_f32(&self.accumulation),
        );
        hashes.insert("discharge".to_string(), self.hash_f32(&self.discharge));
        hashes.insert("flow_dir".to_string(), self.hash_u8(&self.flow_dir));
        hashes.insert(
            "river_class".to_string(),
            self.hash_river_class(&self.river_class),
        );
        hashes.insert("lake_id".to_string(), self.hash_u32(&self.lake_id));
        hashes.insert("ocean_mask".to_string(), self.hash_bool(&self.ocean_mask));
        hashes.insert("biome".to_string(), self.hash_biome(&self.biome));
        hashes.insert("fertility".to_string(), self.hash_f32(&self.fertility));
        hashes.insert(
            "province".to_string(),
            self.hash_province(&self.geologic_province),
        );
        hashes.insert("rock_type".to_string(), self.hash_rock(&self.rock_type));

        for (name, mask) in &self.mineral_masks {
            hashes.insert(format!("mineral_{name}"), self.hash_bool(mask));
        }

        let mut combined = Hasher::new();
        for (name, hash) in &hashes {
            combined.update(name.as_bytes());
            combined.update(hash.as_bytes());
        }
        self.diagnostics.layer_hashes = hashes;
        self.diagnostics.checksum = combined.finalize().to_hex().to_string();
    }

    fn hash_f32(&self, grid: &Grid2D<f32>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&v.to_bits().to_le_bytes());
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_u8(&self, grid: &Grid2D<u8>) -> String {
        let mut h = Hasher::new();
        h.update(grid.as_slice());
        h.finalize().to_hex().to_string()
    }

    fn hash_u32(&self, grid: &Grid2D<u32>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&v.to_le_bytes());
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_bool(&self, grid: &Grid2D<bool>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&[u8::from(*v)]);
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_river_class(&self, grid: &Grid2D<RiverClass>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&[v.as_u8()]);
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_biome(&self, grid: &Grid2D<Biome>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&[v.as_u8()]);
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_province(&self, grid: &Grid2D<GeologicProvince>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&[v.as_u8()]);
        }
        h.finalize().to_hex().to_string()
    }

    fn hash_rock(&self, grid: &Grid2D<RockType>) -> String {
        let mut h = Hasher::new();
        for v in grid.as_slice() {
            h.update(&[v.as_u8()]);
        }
        h.finalize().to_hex().to_string()
    }
}
