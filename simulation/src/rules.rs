use crate::config::{Cap, Condition, Rule};
use crate::state::{has_line, State};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Continue,
    MoverWins,
}

pub fn legal_moves(state: &State, cond: &Condition) -> Vec<u8> {
    let mover = state.to_move();
    let mut occupied = state.cells_of(mover) | state.cells_of(mover.other());
    if cond.rule == Rule::T1 {
        if let Cap::Finite(limit) = cond.cap(mover) {
            let queue = state.cells_in_order(mover);
            if queue.len() == usize::from(limit) {
                occupied &= !(1 << queue[0]);
            }
        }
    }
    (0..9).filter(|cell| occupied & (1 << cell) == 0).collect()
}

pub fn removed_cell(state: &State, cell: u8, cond: &Condition) -> Option<u8> {
    if !legal_moves(state, cond).contains(&cell) {
        return None;
    }
    let mover = state.to_move();
    let Cap::Finite(limit) = cond.cap(mover) else {
        return None;
    };
    let queue = state.cells_in_order(mover);
    (queue.len() == usize::from(limit)).then(|| queue[0])
}

pub fn step(state: &State, cell: u8, cond: &Condition) -> Result<(State, Outcome), String> {
    if !legal_moves(state, cond).contains(&cell) {
        return Err(format!("cell {cell} is not a legal move"));
    }

    let mover = state.to_move();
    let cap = cond.cap(mover);
    let mut cells = state.cells_in_order(mover);
    let outcome = match cond.rule {
        Rule::T1 => {
            remove_oldest_at_capacity(&mut cells, cap);
            cells.push(cell);
            line_outcome(&cells)
        }
        Rule::T2 => {
            cells.push(cell);
            let outcome = line_outcome(&cells);
            remove_oldest_over_capacity(&mut cells, cap);
            outcome
        }
        Rule::T3 => {
            cells.push(cell);
            remove_oldest_over_capacity(&mut cells, cap);
            line_outcome(&cells)
        }
    };
    let next = state.with_cells(mover, &cells, cap).with_to_move(mover.other());
    Ok((next, outcome))
}

fn remove_oldest_at_capacity(cells: &mut Vec<u8>, cap: Cap) {
    if let Cap::Finite(limit) = cap {
        if cells.len() == usize::from(limit) {
            cells.remove(0);
        }
    }
}

fn remove_oldest_over_capacity(cells: &mut Vec<u8>, cap: Cap) {
    if let Cap::Finite(limit) = cap {
        if cells.len() > usize::from(limit) {
            cells.remove(0);
        }
    }
}

fn line_outcome(cells: &[u8]) -> Outcome {
    let mask = cells.iter().fold(0_u16, |mask, cell| mask | (1 << cell));
    if has_line(mask) {
        Outcome::MoverWins
    } else {
        Outcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Cap, Player};

    #[test]
    fn t1_with_cap_one_behaves_like_moving_a_piece() {
        let cond = Condition::new(Rule::T1, Cap::Finite(1), Cap::Finite(1), Player::X);
        let state = State::from_parts(&[0], &[4], Player::X, cond.kx, cond.ko).unwrap();
        let (next, outcome) = step(&state, 8, &cond).unwrap();

        assert_eq!(outcome, Outcome::Continue);
        assert_eq!(next.cells_in_order(Player::X), vec![8]);
        assert!(legal_moves(&state, &cond).contains(&0));
    }
}
