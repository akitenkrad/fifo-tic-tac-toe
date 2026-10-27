use crate::config::{Cap, Player};
use crate::state::State;

pub const D4: [[u8; 9]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8],
    [2, 5, 8, 1, 4, 7, 0, 3, 6],
    [8, 7, 6, 5, 4, 3, 2, 1, 0],
    [6, 3, 0, 7, 4, 1, 8, 5, 2],
    [2, 1, 0, 5, 4, 3, 8, 7, 6],
    [6, 7, 8, 3, 4, 5, 0, 1, 2],
    [0, 3, 6, 1, 4, 7, 2, 5, 8],
    [8, 5, 2, 7, 4, 1, 6, 3, 0],
];

pub fn transform(state: &State, symmetry: usize) -> State {
    let map = &D4[symmetry];
    let x: Vec<_> = state.cells_in_order(Player::X).into_iter().map(|cell| map[usize::from(cell)]).collect();
    let o: Vec<_> = state.cells_in_order(Player::O).into_iter().map(|cell| map[usize::from(cell)]).collect();
    State::from_parts(
        &x,
        &o,
        state.to_move(),
        representation_cap(state, Player::X),
        representation_cap(state, Player::O),
    )
    .expect("a symmetry must preserve a valid state")
}

pub fn canonical(state: &State) -> State {
    (0..D4.len()).map(|symmetry| transform(state, symmetry)).min().expect("D4 is nonempty")
}

pub fn stabilizer(state: &State) -> Vec<usize> {
    (0..D4.len()).filter(|symmetry| transform(state, *symmetry) == *state).collect()
}

fn representation_cap(state: &State, player: Player) -> Cap {
    if state.uses_mask(player) {
        Cap::Inf
    } else {
        Cap::Finite(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_are_eight_distinct_permutations() {
        let mut maps = D4.to_vec();
        maps.sort_unstable();
        maps.dedup();
        assert_eq!(maps.len(), 8);
        for map in D4 {
            let mut cells = map;
            cells.sort_unstable();
            assert_eq!(cells, [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        }
    }
}
