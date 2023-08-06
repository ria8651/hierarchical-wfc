#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

impl Direction {
    pub fn other(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn rotate(&self, rotation: usize) -> Self {
        match rotation {
            0 => *self,
            1 => match self {
                Self::Up => Self::Right,
                Self::Down => Self::Left,
                Self::Left => Self::Up,
                Self::Right => Self::Down,
            },
            2 => match self {
                Self::Up => Self::Down,
                Self::Down => Self::Up,
                Self::Left => Self::Right,
                Self::Right => Self::Left,
            },
            3 => match self {
                Self::Up => Self::Left,
                Self::Down => Self::Right,
                Self::Left => Self::Down,
                Self::Right => Self::Up,
            },
            _ => panic!("Invalid rotation: {}", rotation),
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => panic!("Invalid direction: {}", value),
        }
    }
}
