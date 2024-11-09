use fast_desmos2_comms::{
    value::{List as ValueList, OneRef, Value, ValueKind, ValueRef},
    TypeMismatch,
};

use crate::{executor::evaluator::EvalErrorKind, lexing::Span, math, utils::OptExt};

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum DyadicPervasive {
    Mod,
    Choose,
    Permutation,
    Distance,
}

impl DyadicPervasive {
    pub const fn from_str(from: &[u8]) -> Option<Self> {
        Some(match from {
            b"mod" => Self::Mod,
            b"choose" => Self::Choose,
            b"permuatation" => Self::Permutation,
            b"distance" => Self::Distance,
            _ => return None,
        })
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Choose => "choose",
            Self::Permutation => "permuatation",
            Self::Distance => "distance",
        }
    }

    pub fn type_check(
        &self,
        a: (Span, ValueKind),
        b: (Span, ValueKind),
    ) -> Result<(), EvalErrorKind> {
        match self {
            Self::Mod | Self::Choose | Self::Permutation => {
                a.1.try_number().map_err(EvalErrorKind::wrong_type)?;
                b.1.try_number().map_err(EvalErrorKind::wrong_type)?;
            }
            Self::Distance => match (a.1, b.1) {
                (ValueKind::Number, ValueKind::Number) => {}
                (ValueKind::Point, ValueKind::Point) => {}
                _ => {
                    return Err(EvalErrorKind::TypeMismatch {
                        op_name: self.as_str(),
                        left: a,
                        right: b,
                    })
                }
            },
        }
        Ok(())
    }

    pub fn apply_one(&self, a: OneRef, b: OneRef) -> Result<Value, EvalErrorKind> {
        match self {
            Self::Mod | Self::Choose | Self::Permutation => {
                let &a = a.try_number().unwrap_unreach();
                let &b = b.try_number().unwrap_unreach();

                let result = match self {
                    Self::Mod => a % b,
                    Self::Choose => math::ncr(a, b),
                    Self::Permutation => math::npr(a, b),
                    _ => unreachable!(),
                };

                Ok(Value::one_number(result))
            }
            Self::Distance => match (a, b) {
                (OneRef::Number(&a), OneRef::Number(&b)) => Ok(Value::one_number((a - b).abs())),
                (OneRef::Point(&a), OneRef::Point(&b)) => Ok(Value::one_number((a - b).length())),
                _ => unreachable!(),
            },
        }
    }
}
