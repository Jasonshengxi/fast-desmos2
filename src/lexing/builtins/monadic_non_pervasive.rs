use fast_desmos2_comms::Value;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MonadicNonPervasive {
    Length,
    Unique,
}

impl MonadicNonPervasive {
    pub const fn from_str(from: &[u8]) -> Option<Self> {
        Some(match from {
            b"length" => Self::Length,
            b"unique" => Self::Unique,
            _ => return None,
        })
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Unique => "unique",
        }
    }

    pub fn apply(&self, x: Value) -> Value {
        match self {
            Self::Length => Value::one_number(x.len().map(|x| x as f64).unwrap_or(1.0)),
            Self::Unique => x.unique(),
        }
    }
}
