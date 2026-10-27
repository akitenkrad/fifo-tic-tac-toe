use crate::config::{Cap, Player};

pub const LINES: [u16; 8] = [
    0b000_000_111,
    0b000_111_000,
    0b111_000_000,
    0b001_001_001,
    0b010_010_010,
    0b100_100_100,
    0b100_010_001,
    0b001_010_100,
];

const PLAYER_BITS: u32 = 20;
const LEN_SHIFT: u32 = 16;
const INF_SHIFT: u32 = 19;
const PLAYER_MASK: u64 = (1 << PLAYER_BITS) - 1;
const TURN_SHIFT: u32 = 40;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct State(pub u64);

impl State {
    pub const fn initial(first: Player) -> Self {
        Self((matches!(first, Player::O) as u64) << TURN_SHIFT)
    }

    pub fn from_parts(x: &[u8], o: &[u8], to_move: Player, kx: Cap, ko: Cap) -> Result<Self, String> {
        validate_cells(x, kx)?;
        validate_cells(o, ko)?;
        let x_mask = cells_mask(x)?;
        let o_mask = cells_mask(o)?;
        if x_mask & o_mask != 0 {
            return Err("players cannot occupy the same cell".to_owned());
        }
        let mut state = Self::initial(to_move);
        state.set_cells(Player::X, x, kx);
        state.set_cells(Player::O, o, ko);
        Ok(state)
    }

    pub const fn packed(self) -> u64 {
        self.0
    }

    pub const fn to_move(self) -> Player {
        if self.0 & (1 << TURN_SHIFT) == 0 {
            Player::X
        } else {
            Player::O
        }
    }

    pub fn cells_of(self, player: Player) -> u16 {
        let part = self.player_part(player);
        if part & (1 << INF_SHIFT) != 0 {
            (part & 0x1ff) as u16
        } else {
            self.cells_in_order(player).into_iter().fold(0, |mask, cell| mask | (1 << cell))
        }
    }

    pub fn cells_in_order(self, player: Player) -> Vec<u8> {
        let part = self.player_part(player);
        if part & (1 << INF_SHIFT) != 0 {
            let mask = (part & 0x1ff) as u16;
            return (0..9).filter(|cell| mask & (1 << cell) != 0).collect();
        }
        let len = ((part >> LEN_SHIFT) & 0x7) as usize;
        (0..len).map(|index| ((part >> (index * 4)) & 0xf) as u8).collect()
    }

    pub const fn uses_mask(self, player: Player) -> bool {
        let shift = match player {
            Player::X => 0,
            Player::O => PLAYER_BITS,
        };
        ((self.0 >> shift) & (1 << INF_SHIFT)) != 0
    }

    pub(crate) fn with_cells(mut self, player: Player, cells: &[u8], cap: Cap) -> Self {
        self.set_cells(player, cells, cap);
        self
    }

    pub(crate) fn with_to_move(mut self, player: Player) -> Self {
        self.0 &= !(1 << TURN_SHIFT);
        self.0 |= (matches!(player, Player::O) as u64) << TURN_SHIFT;
        self
    }

    fn player_part(self, player: Player) -> u64 {
        (self.0 >> player_shift(player)) & PLAYER_MASK
    }

    fn set_cells(&mut self, player: Player, cells: &[u8], cap: Cap) {
        let shift = player_shift(player);
        self.0 &= !(PLAYER_MASK << shift);
        let part = match cap {
            Cap::Inf => {
                let mask = cells.iter().fold(0_u64, |mask, cell| mask | (1 << cell));
                (1 << INF_SHIFT) | mask
            }
            Cap::Finite(_) => {
                let data = cells
                    .iter()
                    .enumerate()
                    .fold(0_u64, |data, (index, cell)| data | (u64::from(*cell) << (index * 4)));
                data | ((cells.len() as u64) << LEN_SHIFT)
            }
        };
        self.0 |= part << shift;
    }
}

pub const fn has_line(mask: u16) -> bool {
    let mut index = 0;
    while index < LINES.len() {
        if mask & LINES[index] == LINES[index] {
            return true;
        }
        index += 1;
    }
    false
}

fn player_shift(player: Player) -> u32 {
    match player {
        Player::X => 0,
        Player::O => PLAYER_BITS,
    }
}

fn validate_cells(cells: &[u8], cap: Cap) -> Result<(), String> {
    if cells.iter().any(|cell| *cell >= 9) {
        return Err("cell must be between 0 and 8".to_owned());
    }
    let mask = cells_mask(cells)?;
    if mask.count_ones() as usize != cells.len() {
        return Err("a player cannot occupy a cell more than once".to_owned());
    }
    if let Cap::Finite(limit) = cap {
        if !(1..=4).contains(&limit) {
            return Err("finite cap must be between 1 and 4".to_owned());
        }
        if cells.len() > usize::from(limit) {
            return Err("cell queue exceeds its finite cap".to_owned());
        }
    }
    Ok(())
}

fn cells_mask(cells: &[u8]) -> Result<u16, String> {
    cells.iter().try_fold(0_u16, |mask, cell| {
        if *cell >= 9 {
            Err("cell must be between 0 and 8".to_owned())
        } else {
            Ok(mask | (1 << cell))
        }
    })
}
