use std::collections::HashMap;

use socsim_core::{AgentId, SimClock, WorldState};

use crate::config::{Condition, Player};
use crate::state::State;

pub const X_AGENT: AgentId = AgentId(0);
pub const O_AGENT: AgentId = AgentId(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameResult {
    XWin,
    OWin,
    Draw,
}

impl GameResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XWin => "x_win",
            Self::OWin => "o_win",
            Self::Draw => "draw",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndReason {
    Line,
    NoMove,
    Repeat,
    Cap,
}

impl EndReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::NoMove => "no_move",
            Self::Repeat => "repeat",
            Self::Cap => "cap",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameEnd {
    pub result: GameResult,
    pub plies: u64,
    pub reason: EndReason,
    pub repeat_to: Option<u64>,
    pub final_state: State,
}

pub struct GameWorld {
    pub state: State,
    pub cond: Condition,
    pub seen: HashMap<State, u64>,
    pub unit_id: String,
    pub max_plies: u64,
    pub stop_on_repeat: bool,
    clock: SimClock,
    end: Option<GameEnd>,
}

impl GameWorld {
    pub fn new(cond: Condition, unit_id: impl Into<String>, max_plies: u64, stop_on_repeat: bool) -> Self {
        Self::from_state(cond, State::initial(cond.first), unit_id, max_plies, stop_on_repeat)
    }

    pub fn from_state(cond: Condition, state: State, unit_id: impl Into<String>, max_plies: u64, stop_on_repeat: bool) -> Self {
        let mut seen = HashMap::new();
        seen.insert(state, 0);
        Self {
            state,
            cond,
            seen,
            unit_id: unit_id.into(),
            max_plies,
            stop_on_repeat,
            clock: SimClock::new(max_plies),
            end: None,
        }
    }

    pub const fn to_move(&self) -> Player {
        self.state.to_move()
    }

    pub const fn end(&self) -> Option<&GameEnd> {
        self.end.as_ref()
    }

    pub(crate) fn finish(&mut self, end: GameEnd) {
        self.end = Some(end);
    }
}

impl WorldState for GameWorld {
    fn agent_ids(&self) -> Vec<AgentId> {
        vec![X_AGENT, O_AGENT]
    }

    fn clock(&self) -> &SimClock {
        &self.clock
    }

    fn clock_mut(&mut self) -> &mut SimClock {
        &mut self.clock
    }
}
