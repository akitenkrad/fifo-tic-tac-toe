use serde_json::json;
use socsim_core::{Mechanism, Phase, Result as SocsimResult, SocsimError, StepContext};

use crate::config::Player;
use crate::policy::{Observation, Policy};
use crate::rules::{legal_moves, removed_cell, step, Outcome};
use crate::world::{EndReason, GameEnd, GameResult, GameWorld};

const CHOSEN_MOVE: &str = "fifo_ttt.chosen_move";
const APPLIED_MOVE: &str = "fifo_ttt.applied_move";

pub struct ChooseMove {
    x: Box<dyn Policy>,
    o: Box<dyn Policy>,
}

impl ChooseMove {
    pub fn new(x: Box<dyn Policy>, o: Box<dyn Policy>) -> Self {
        Self { x, o }
    }
}

impl Mechanism<GameWorld> for ChooseMove {
    fn name(&self) -> &str {
        "choose_move"
    }

    fn phases(&self) -> &'static [Phase] {
        &[Phase::Decision]
    }

    fn apply(&mut self, _phase: Phase, ctx: &mut StepContext<'_, GameWorld>) -> SocsimResult<()> {
        if ctx.world.end().is_some() {
            return Ok(());
        }
        let mover = ctx.world.to_move();
        let legal = legal_moves(&ctx.world.state, &ctx.world.cond);
        if legal.is_empty() {
            return Err(SocsimError::Mechanism("the player to move has no legal move".to_owned()));
        }
        let observation = Observation {
            state: ctx.world.state,
            cond: ctx.world.cond,
            legal,
        };
        let selected = match mover {
            Player::X => self.x.choose(&observation, ctx.rng),
            Player::O => self.o.choose(&observation, ctx.rng),
        };
        if !observation.legal.contains(&selected) {
            return Err(SocsimError::Mechanism(format!(
                "policy returned illegal move {selected} for player {mover}"
            )));
        }
        ctx.scratch.insert(CHOSEN_MOVE, selected);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct AppliedMove {
    mover: Player,
    outcome: Outcome,
}

pub struct ApplyMove;

impl Mechanism<GameWorld> for ApplyMove {
    fn name(&self) -> &str {
        "apply_move"
    }

    fn phases(&self) -> &'static [Phase] {
        &[Phase::Interaction]
    }

    fn apply(&mut self, _phase: Phase, ctx: &mut StepContext<'_, GameWorld>) -> SocsimResult<()> {
        if ctx.world.end().is_some() {
            return Ok(());
        }
        let cell = ctx
            .scratch
            .get::<u8>(CHOSEN_MOVE)
            .copied()
            .ok_or_else(|| SocsimError::Mechanism("no move was selected".to_owned()))?;
        let mover = ctx.world.to_move();
        let removed = removed_cell(&ctx.world.state, cell, &ctx.world.cond);
        let (next, outcome) = step(&ctx.world.state, cell, &ctx.world.cond).map_err(SocsimError::Mechanism)?;
        ctx.world.state = next;
        ctx.recorder.record_event(
            ctx.clock.t(),
            "ply",
            json!({
                "unit_id": ctx.world.unit_id,
                "player": mover.to_string(),
                "cell": cell,
                "removed": removed,
            }),
        );
        ctx.scratch.insert(APPLIED_MOVE, AppliedMove { mover, outcome });
        Ok(())
    }
}

pub struct Referee;

impl Referee {
    fn finish(ctx: &mut StepContext<'_, GameWorld>, result: GameResult, reason: EndReason, repeat_to: Option<u64>) {
        let end = GameEnd {
            result,
            plies: ctx.clock.t(),
            reason,
            repeat_to,
            final_state: ctx.world.state,
        };
        ctx.recorder.record_event(
            ctx.clock.t(),
            "game_end",
            json!({
                "unit_id": ctx.world.unit_id,
                "result": result.as_str(),
                "plies": ctx.clock.t(),
                "reason": reason.as_str(),
                "repeat_to": repeat_to,
            }),
        );
        ctx.world.finish(end);
        ctx.request_stop();
    }
}

impl Mechanism<GameWorld> for Referee {
    fn name(&self) -> &str {
        "referee"
    }

    fn phases(&self) -> &'static [Phase] {
        &[Phase::PostStep]
    }

    fn apply(&mut self, _phase: Phase, ctx: &mut StepContext<'_, GameWorld>) -> SocsimResult<()> {
        if ctx.world.end().is_some() {
            return Ok(());
        }
        let applied = ctx
            .scratch
            .get::<AppliedMove>(APPLIED_MOVE)
            .copied()
            .ok_or_else(|| SocsimError::Mechanism("no move was applied".to_owned()))?;
        if applied.outcome == Outcome::MoverWins {
            let result = match applied.mover {
                Player::X => GameResult::XWin,
                Player::O => GameResult::OWin,
            };
            Self::finish(ctx, result, EndReason::Line, None);
            return Ok(());
        }
        if legal_moves(&ctx.world.state, &ctx.world.cond).is_empty() {
            Self::finish(ctx, GameResult::Draw, EndReason::NoMove, None);
            return Ok(());
        }
        if ctx.world.stop_on_repeat {
            if let Some(repeat_to) = ctx.world.seen.get(&ctx.world.state).copied() {
                Self::finish(ctx, GameResult::Draw, EndReason::Repeat, Some(repeat_to));
                return Ok(());
            }
        }
        if ctx.clock.t() >= ctx.world.max_plies {
            Self::finish(ctx, GameResult::Draw, EndReason::Cap, None);
            return Ok(());
        }
        ctx.world.seen.entry(ctx.world.state).or_insert(ctx.clock.t());
        Ok(())
    }
}
