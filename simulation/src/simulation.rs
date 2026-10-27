use socsim_core::Recorder;
use socsim_engine::{SequentialScheduler, SimulationBuilder};

use crate::config::Condition;
use crate::mechanisms::{ApplyMove, ChooseMove, Referee};
use crate::policy::Policy;
use crate::world::{GameEnd, GameWorld};

#[derive(Clone, Debug)]
pub struct PlayOptions {
    pub unit_id: String,
    pub max_plies: u64,
    pub stop_on_repeat: bool,
}

impl PlayOptions {
    pub fn new(unit_id: impl Into<String>, max_plies: u64, stop_on_repeat: bool) -> Self {
        Self {
            unit_id: unit_id.into(),
            max_plies,
            stop_on_repeat,
        }
    }
}

pub fn play_game(
    cond: Condition,
    x: Box<dyn Policy>,
    o: Box<dyn Policy>,
    recorder: Box<dyn Recorder>,
    seed: u64,
    opts: PlayOptions,
) -> Result<GameEnd, String> {
    if opts.max_plies == 0 {
        return Err("max_plies must be at least one".to_owned());
    }
    let world = GameWorld::new(cond, opts.unit_id, opts.max_plies, opts.stop_on_repeat);
    let mut simulation = SimulationBuilder::new(world)
        .scheduler(Box::new(SequentialScheduler))
        .seed(seed)
        .add_mechanism(Box::new(ChooseMove::new(x, o)))
        .add_mechanism(Box::new(ApplyMove))
        .add_mechanism(Box::new(Referee))
        .recorder(recorder)
        .build();
    simulation.run().map_err(|error| error.to_string())?;
    simulation
        .world()
        .end()
        .cloned()
        .ok_or_else(|| "game ended without a referee result".to_owned())
}
