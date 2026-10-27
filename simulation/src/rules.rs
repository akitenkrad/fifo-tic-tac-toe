use crate::config::{Cap, Condition, Rule};
use crate::state::{has_line, State};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Continue,
    MoverWins,
}

pub fn legal_moves(state: &State, cond: &Condition) -> Vec<u8> {
    let available = legal_move_mask(state, cond);
    (0..9).filter(|cell| available & (1 << cell) != 0).collect()
}

fn legal_move_mask(state: &State, cond: &Condition) -> u16 {
    let mover = state.to_move();
    let mut occupied = state.cells_of(mover) | state.cells_of(mover.other());
    if cond.rule == Rule::T1 {
        if let Cap::Finite(limit) = cond.cap(mover) {
            if state.cell_count(mover) == usize::from(limit) {
                let oldest = state.oldest_cell(mover).expect("a player at capacity has an oldest cell");
                occupied &= !(1 << oldest);
            }
        }
    }
    !occupied & 0x1ff
}

pub fn removed_cell(state: &State, cell: u8, cond: &Condition) -> Option<u8> {
    if cell >= 9 || legal_move_mask(state, cond) & (1 << cell) == 0 {
        return None;
    }
    let mover = state.to_move();
    let Cap::Finite(limit) = cond.cap(mover) else {
        return None;
    };
    (state.cell_count(mover) == usize::from(limit)).then(|| state.oldest_cell(mover).expect("a player at capacity has an oldest cell"))
}

pub fn step(state: &State, cell: u8, cond: &Condition) -> Result<(State, Outcome), String> {
    if cell >= 9 || legal_move_mask(state, cond) & (1 << cell) == 0 {
        return Err(format!("cell {cell} is not a legal move"));
    }

    let mover = state.to_move();
    let cap = cond.cap(mover);
    let mut cells = [0_u8; 10];
    let mut len = state.copy_cells(mover, &mut cells);
    let outcome = match cond.rule {
        Rule::T1 => {
            remove_oldest_at_capacity(&mut cells, &mut len, cap);
            cells[len] = cell;
            len += 1;
            line_outcome(&cells[..len])
        }
        Rule::T2 => {
            cells[len] = cell;
            len += 1;
            let outcome = line_outcome(&cells[..len]);
            remove_oldest_over_capacity(&mut cells, &mut len, cap);
            outcome
        }
        Rule::T3 => {
            cells[len] = cell;
            len += 1;
            remove_oldest_over_capacity(&mut cells, &mut len, cap);
            line_outcome(&cells[..len])
        }
    };
    let next = state.with_cells(mover, &cells[..len], cap).with_to_move(mover.other());
    Ok((next, outcome))
}

fn remove_oldest_at_capacity(cells: &mut [u8; 10], len: &mut usize, cap: Cap) {
    if let Cap::Finite(limit) = cap {
        if *len == usize::from(limit) {
            cells.copy_within(1..*len, 0);
            *len -= 1;
        }
    }
}

fn remove_oldest_over_capacity(cells: &mut [u8; 10], len: &mut usize, cap: Cap) {
    if let Cap::Finite(limit) = cap {
        if *len > usize::from(limit) {
            cells.copy_within(1..*len, 0);
            *len -= 1;
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
