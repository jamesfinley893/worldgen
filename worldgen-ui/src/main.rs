use std::path::PathBuf;

use eframe::egui;
use worldgen_core::export::export_snapshot;
use worldgen_core::state::{Biome, GeologicProvince, RiverClass, RockType, Step};
use worldgen_core::{
    run_all_steps, run_next_step, run_step, GenerationParams, MapSizePreset, WorldState,
};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DF-Style Worldgen",
        options,
        Box::new(|_cc| Ok(Box::new(WorldgenApp::default()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewLayer {
    Elevation,
    Temperature,
    Rainfall,
    Accumulation,
    Discharge,
    FlowDir,
    RiverClass,
    Lake,
    OceanMask,
    Biome,
    Fertility,
    Province,
    RockType,
    MineralIron,
    MineralCopper,
    MineralGold,
    MineralTin,
    MineralCoal,
    MineralGem,
}

impl ViewLayer {
    const ALL: [Self; 19] = [
        Self::Elevation,
        Self::Temperature,
        Self::Rainfall,
        Self::Accumulation,
        Self::Discharge,
        Self::FlowDir,
        Self::RiverClass,
        Self::Lake,
        Self::OceanMask,
        Self::Biome,
        Self::Fertility,
        Self::Province,
        Self::RockType,
        Self::MineralIron,
        Self::MineralCopper,
        Self::MineralGold,
        Self::MineralTin,
        Self::MineralCoal,
        Self::MineralGem,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Elevation => "Elevation",
            Self::Temperature => "Temperature",
            Self::Rainfall => "Rainfall",
            Self::Accumulation => "Accumulation",
            Self::Discharge => "Discharge",
            Self::FlowDir => "Flow Dir",
            Self::RiverClass => "River Class",
            Self::Lake => "Lake ID",
            Self::OceanMask => "Ocean Mask",
            Self::Biome => "Biome",
            Self::Fertility => "Fertility",
            Self::Province => "Geologic Province",
            Self::RockType => "Rock Type",
            Self::MineralIron => "Mineral Iron",
            Self::MineralCopper => "Mineral Copper",
            Self::MineralGold => "Mineral Gold",
            Self::MineralTin => "Mineral Tin",
            Self::MineralCoal => "Mineral Coal",
            Self::MineralGem => "Mineral Gem",
        }
    }
}

struct WorldgenApp {
    params: GenerationParams,
    state: WorldState,
    view_layer: ViewLayer,
    texture: Option<egui::TextureHandle>,
    last_error: Option<String>,
}

impl Default for WorldgenApp {
    fn default() -> Self {
        let params = GenerationParams::default();
        let state = WorldState::new(params.clone());
        Self {
            params,
            state,
            view_layer: ViewLayer::Elevation,
            texture: None,
            last_error: None,
        }
    }
}

impl eframe::App for WorldgenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("controls")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Worldgen Controls");

                ui.horizontal(|ui| {
                    ui.label("Seed");
                    ui.add(egui::DragValue::new(&mut self.params.seed).speed(1));
                });

                egui::ComboBox::from_label("Size")
                    .selected_text(match self.params.size {
                        MapSizePreset::S256 => "256x256",
                        MapSizePreset::S512 => "512x512",
                        MapSizePreset::S1024 => "1024x1024",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.params.size, MapSizePreset::S256, "256x256");
                        ui.selectable_value(&mut self.params.size, MapSizePreset::S512, "512x512");
                        ui.selectable_value(
                            &mut self.params.size,
                            MapSizePreset::S1024,
                            "1024x1024",
                        );
                    });

                ui.separator();
                ui.collapsing("Step 1: Base", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.base.sea_level, 0.2..=0.8)
                            .text("Sea Level"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.base.frequency, 0.5..=4.0)
                            .text("Frequency"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.base.warp_strength, 0.0..=0.12)
                            .text("Warp"),
                    );
                });

                ui.collapsing("Step 2: Erosion", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.erosion.iterations, 1..=80)
                            .text("Iterations"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.erosion.erosion_rate, 0.0..=0.1)
                            .text("Erosion"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.erosion.thermal_rate, 0.0..=0.08)
                            .text("Thermal"),
                    );
                });

                ui.collapsing("Step 3: Biomes", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.biomes.smoothing_passes, 0..=6)
                            .text("Smooth Passes"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.biomes.wetness_weight, 0.0..=1.0)
                            .text("Wetness Weight"),
                    );
                });

                ui.collapsing("Step 4: Hydro Final", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.hydro.ephemeral_threshold, 10.0..=200.0)
                            .text("Ephemeral Q"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.hydro.perennial_threshold, 40.0..=600.0)
                            .text("Perennial Q"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.hydro.major_threshold, 120.0..=2000.0)
                            .text("Major Q"),
                    );
                });

                ui.collapsing("Step 5: Geology", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.geology.strata_layers, 3..=10)
                            .text("Strata Layers"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.geology.fault_strength, 0.0..=1.0)
                            .text("Fault Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.geology.ore_richness, 0.0..=1.0)
                            .text("Ore Richness"),
                    );
                });

                ui.separator();
                if ui.button("Generate").clicked() {
                    self.regenerate(Step::BaseFields, true);
                    self.refresh_texture(ctx);
                }
                if ui.button("Run Step").clicked() {
                    self.run_step_button(false);
                    self.refresh_texture(ctx);
                }
                if ui.button("Run All").clicked() {
                    self.run_all_button(false);
                    self.refresh_texture(ctx);
                }
                if ui.button("Export Snapshot").clicked() {
                    match self.export_snapshot() {
                        Ok(path) => {
                            self.last_error = Some(format!("Exported to {}", path.display()));
                        }
                        Err(e) => {
                            self.last_error = Some(format!("Export failed: {e}"));
                        }
                    }
                }

                ui.separator();
                egui::ComboBox::from_label("Layer")
                    .selected_text(self.view_layer.label())
                    .show_ui(ui, |ui| {
                        for layer in ViewLayer::ALL {
                            if ui
                                .selectable_value(&mut self.view_layer, layer, layer.label())
                                .clicked()
                            {
                                self.refresh_texture(ctx);
                            }
                        }
                    });

                let step_text = match self.state.current_step {
                    Some(step) => format!("{}/5 ({:?})", step.index(), step),
                    None => "0/5 (Not started)".to_string(),
                };
                ui.label(format!("Step: {step_text}"));
                ui.label(format!("Checksum: {}", self.state.diagnostics.checksum));

                for s in Step::ALL {
                    let ms = self.state.step_timings_ms.get(&s).copied().unwrap_or(0.0);
                    ui.label(format!("{:?}: {:.2} ms", s, ms));
                }

                if let Some(msg) = &self.last_error {
                    ui.separator();
                    ui.label(msg);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.texture.is_none() {
                self.refresh_texture(ctx);
            }
            if let Some(tex) = &self.texture {
                let avail = ui.available_size();
                let mut size = tex.size_vec2();
                if size.x > avail.x {
                    let k = avail.x / size.x;
                    size *= k;
                }
                if size.y > avail.y {
                    let k = avail.y / size.y;
                    size *= k;
                }
                ui.image((tex.id(), size));
            }
        });
    }
}

impl WorldgenApp {
    fn regenerate(&mut self, first_step: Step, run_one: bool) {
        self.state = WorldState::new(self.params.clone());
        self.state.params = self.params.clone();
        self.last_error = None;
        if run_one {
            if let Err(e) = run_step(&mut self.state, first_step, &self.params) {
                self.last_error = Some(format!("Generate failed: {e}"));
            }
        }
    }

    fn run_step_button(&mut self, reset: bool) {
        if reset {
            self.regenerate(Step::BaseFields, false);
        }
        match run_next_step(&mut self.state, &self.params) {
            Ok(Some(_)) => {}
            Ok(None) => self.last_error = Some("All steps are already complete".to_string()),
            Err(e) => self.last_error = Some(format!("Run Step failed: {e}")),
        }
    }

    fn run_all_button(&mut self, reset: bool) {
        if reset {
            self.regenerate(Step::BaseFields, false);
        }
        if let Err(e) = run_all_steps(&mut self.state, &self.params) {
            self.last_error = Some(format!("Run All failed: {e}"));
        }
    }

    fn export_snapshot(&self) -> anyhow::Result<PathBuf> {
        let checksum_prefix = self
            .state
            .diagnostics
            .checksum
            .chars()
            .take(8)
            .collect::<String>();
        let dir =
            PathBuf::from("exports").join(format!("seed{}_{}", self.params.seed, checksum_prefix));
        export_snapshot(&self.state, &dir)?;
        Ok(dir)
    }

    fn refresh_texture(&mut self, ctx: &egui::Context) {
        let image = layer_image(&self.state, self.view_layer);
        self.texture = Some(ctx.load_texture("world-layer", image, Default::default()));
    }
}

fn layer_image(state: &WorldState, layer: ViewLayer) -> egui::ColorImage {
    let mut pixels = vec![egui::Color32::BLACK; state.width * state.height];

    let to_idx = |x: usize, y: usize| y * state.width + x;

    let float_norm = |v: f32, min: f32, max: f32| {
        let span = (max - min).max(1e-9);
        ((v - min) / span).clamp(0.0, 1.0)
    };

    let (fmin, fmax) = match layer {
        ViewLayer::Elevation => min_max(state.elevation.as_slice()),
        ViewLayer::Temperature => min_max(state.temperature.as_slice()),
        ViewLayer::Rainfall => min_max(state.rainfall.as_slice()),
        ViewLayer::Accumulation => min_max(state.accumulation.as_slice()),
        ViewLayer::Discharge => min_max(state.discharge.as_slice()),
        ViewLayer::Fertility => min_max(state.fertility.as_slice()),
        _ => (0.0, 1.0),
    };

    for y in 0..state.height {
        for x in 0..state.width {
            let color = match layer {
                ViewLayer::Elevation => {
                    let v = float_norm(*state.elevation.get(x, y), fmin, fmax);
                    egui::Color32::from_rgb((v * 255.0) as u8, (v * 255.0) as u8, (v * 255.0) as u8)
                }
                ViewLayer::Temperature => {
                    let v = float_norm(*state.temperature.get(x, y), fmin, fmax);
                    egui::Color32::from_rgb((v * 255.0) as u8, 60, ((1.0 - v) * 255.0) as u8)
                }
                ViewLayer::Rainfall => {
                    let v = float_norm(*state.rainfall.get(x, y), fmin, fmax);
                    egui::Color32::from_rgb(20, (v * 255.0) as u8, 200)
                }
                ViewLayer::Accumulation => {
                    let v = float_norm(
                        (*state.accumulation.get(x, y) + 1.0).ln(),
                        (fmin + 1.0).ln(),
                        (fmax + 1.0).ln(),
                    );
                    egui::Color32::from_rgb(0, (v * 255.0) as u8, 255)
                }
                ViewLayer::Discharge => {
                    let v = float_norm(
                        (*state.discharge.get(x, y) + 1.0).ln(),
                        (fmin + 1.0).ln(),
                        (fmax + 1.0).ln(),
                    );
                    egui::Color32::from_rgb(0, (v * 200.0) as u8, 255)
                }
                ViewLayer::FlowDir => {
                    let d = *state.flow_dir.get(x, y);
                    if d == 255 {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::from_rgb(d.wrapping_mul(28), 200, 120)
                    }
                }
                ViewLayer::RiverClass => match *state.river_class.get(x, y) {
                    RiverClass::None => egui::Color32::BLACK,
                    RiverClass::Ephemeral => egui::Color32::from_rgb(90, 170, 255),
                    RiverClass::Perennial => egui::Color32::from_rgb(40, 130, 245),
                    RiverClass::Major => egui::Color32::from_rgb(0, 60, 220),
                },
                ViewLayer::Lake => {
                    let id = *state.lake_id.get(x, y);
                    if id == 0 {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::from_rgb(
                            (id.wrapping_mul(53) % 255) as u8,
                            (id.wrapping_mul(97) % 255) as u8,
                            (id.wrapping_mul(191) % 255) as u8,
                        )
                    }
                }
                ViewLayer::OceanMask => {
                    if *state.ocean_mask.get(x, y) {
                        egui::Color32::from_rgb(0, 50, 130)
                    } else {
                        egui::Color32::from_rgb(20, 20, 20)
                    }
                }
                ViewLayer::Biome => biome_color(*state.biome.get(x, y)),
                ViewLayer::Fertility => {
                    let v = float_norm(*state.fertility.get(x, y), fmin, fmax);
                    egui::Color32::from_rgb((v * 140.0) as u8, (v * 255.0) as u8, 70)
                }
                ViewLayer::Province => province_color(*state.geologic_province.get(x, y)),
                ViewLayer::RockType => rock_color(*state.rock_type.get(x, y)),
                ViewLayer::MineralIron => mineral_color(state, x, y, "iron"),
                ViewLayer::MineralCopper => mineral_color(state, x, y, "copper"),
                ViewLayer::MineralGold => mineral_color(state, x, y, "gold"),
                ViewLayer::MineralTin => mineral_color(state, x, y, "tin"),
                ViewLayer::MineralCoal => mineral_color(state, x, y, "coal"),
                ViewLayer::MineralGem => mineral_color(state, x, y, "gem"),
            };
            pixels[to_idx(x, y)] = color;
        }
    }

    egui::ColorImage {
        size: [state.width, state.height],
        pixels,
    }
}

fn min_max(slice: &[f32]) -> (f32, f32) {
    let min = slice.iter().copied().fold(f32::INFINITY, f32::min);
    let max = slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}

fn biome_color(b: Biome) -> egui::Color32 {
    match b {
        Biome::Ocean => egui::Color32::from_rgb(18, 52, 120),
        Biome::Lake => egui::Color32::from_rgb(42, 98, 190),
        Biome::PolarDesert => egui::Color32::from_rgb(230, 230, 218),
        Biome::Tundra => egui::Color32::from_rgb(176, 190, 156),
        Biome::BorealForest => egui::Color32::from_rgb(66, 102, 63),
        Biome::TemperateGrassland => egui::Color32::from_rgb(158, 186, 102),
        Biome::TemperateForest => egui::Color32::from_rgb(74, 136, 80),
        Biome::Mediterranean => egui::Color32::from_rgb(146, 152, 84),
        Biome::Savanna => egui::Color32::from_rgb(186, 172, 78),
        Biome::TropicalSeasonalForest => egui::Color32::from_rgb(88, 144, 70),
        Biome::TropicalRainforest => egui::Color32::from_rgb(42, 116, 52),
        Biome::HotDesert => egui::Color32::from_rgb(214, 188, 126),
        Biome::Alpine => egui::Color32::from_rgb(136, 136, 140),
        Biome::Wetland => egui::Color32::from_rgb(76, 138, 120),
    }
}

fn province_color(p: GeologicProvince) -> egui::Color32 {
    match p {
        GeologicProvince::Oceanic => egui::Color32::from_rgb(30, 80, 130),
        GeologicProvince::Craton => egui::Color32::from_rgb(132, 112, 95),
        GeologicProvince::Orogen => egui::Color32::from_rgb(170, 110, 90),
        GeologicProvince::Basin => egui::Color32::from_rgb(100, 130, 95),
        GeologicProvince::VolcanicArc => egui::Color32::from_rgb(182, 70, 52),
    }
}

fn rock_color(r: RockType) -> egui::Color32 {
    match r {
        RockType::Basalt => egui::Color32::from_rgb(65, 65, 72),
        RockType::Gabbro => egui::Color32::from_rgb(50, 58, 66),
        RockType::Granite => egui::Color32::from_rgb(186, 175, 164),
        RockType::Sandstone => egui::Color32::from_rgb(194, 168, 120),
        RockType::Limestone => egui::Color32::from_rgb(204, 198, 160),
        RockType::Schist => egui::Color32::from_rgb(130, 122, 140),
        RockType::Gneiss => egui::Color32::from_rgb(144, 136, 128),
        RockType::Shale => egui::Color32::from_rgb(104, 102, 94),
        RockType::Rhyolite => egui::Color32::from_rgb(176, 130, 120),
    }
}

fn mineral_color(state: &WorldState, x: usize, y: usize, key: &str) -> egui::Color32 {
    if let Some(mask) = state.mineral_masks.get(key) {
        if *mask.get(x, y) {
            return egui::Color32::from_rgb(240, 200, 30);
        }
    }
    egui::Color32::BLACK
}
