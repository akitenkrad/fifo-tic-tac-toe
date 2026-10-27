use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub const fn other(self) -> Self {
        match self {
            Self::X => Self::O,
            Self::O => Self::X,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::X => "x",
            Self::O => "o",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Rule {
    T1,
    T2,
    T3,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::T1 => "t1",
            Self::T2 => "t2",
            Self::T3 => "t3",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Cap {
    Finite(u8),
    Inf,
}

impl Cap {
    pub const fn finite(self) -> Option<u8> {
        match self {
            Self::Finite(value) => Some(value),
            Self::Inf => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Finite(1) => "1",
            Self::Finite(2) => "2",
            Self::Finite(3) => "3",
            Self::Finite(4) => "4",
            Self::Finite(_) => "invalid",
            Self::Inf => "inf",
        }
    }
}

pub const K_VALUES: [Cap; 5] = [Cap::Finite(1), Cap::Finite(2), Cap::Finite(3), Cap::Finite(4), Cap::Inf];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Condition {
    pub rule: Rule,
    pub kx: Cap,
    pub ko: Cap,
    pub first: Player,
    pub horizon: Option<u32>,
}

impl Condition {
    pub const fn new(rule: Rule, kx: Cap, ko: Cap, first: Player) -> Self {
        Self {
            rule,
            kx,
            ko,
            first,
            horizon: None,
        }
    }

    pub const fn cap(self, player: Player) -> Cap {
        match player {
            Player::X => self.kx,
            Player::O => self.ko,
        }
    }
}
