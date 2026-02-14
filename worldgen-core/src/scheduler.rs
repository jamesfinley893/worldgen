use anyhow::Result;

use crate::params::GenerationParams;
use crate::state::{Step, WorldState};
use crate::systems;
use crate::time::StepTimer;

pub fn run_step(state: &mut WorldState, step: Step, params: &GenerationParams) -> Result<()> {
    state.params = params.clone();
    let timer = StepTimer::start();
    match step {
        Step::BaseFields => systems::base_fields::run(state, params),
        Step::ErosionHydrology => systems::erosion_hydrology::run(state, params),
        Step::Biomes => systems::biomes::run(state, params),
        Step::HydroFinalize => systems::hydro_finalize::run(state, params),
        Step::Geology => systems::geology::run(state, params),
    }
    state.current_step = Some(step);
    state.step_timings_ms.insert(step, timer.elapsed_ms());
    state.update_diagnostics();
    Ok(())
}

pub fn run_next_step(state: &mut WorldState, params: &GenerationParams) -> Result<Option<Step>> {
    let next = match state.current_step {
        None => Some(Step::BaseFields),
        Some(Step::BaseFields) => Some(Step::ErosionHydrology),
        Some(Step::ErosionHydrology) => Some(Step::Biomes),
        Some(Step::Biomes) => Some(Step::HydroFinalize),
        Some(Step::HydroFinalize) => Some(Step::Geology),
        Some(Step::Geology) => None,
    };

    if let Some(step) = next {
        run_step(state, step, params)?;
    }
    Ok(next)
}

pub fn run_all_steps(state: &mut WorldState, params: &GenerationParams) -> Result<()> {
    for step in Step::ALL {
        run_step(state, step, params)?;
    }
    Ok(())
}
