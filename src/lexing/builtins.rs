use fast_desmos2_comms::{List as ValueList, TypeMismatch, Value};

mod dyadic_pervasive;
mod monadic_pervasive;

pub use dyadic_pervasive::DyadicPervasive;
pub use monadic_pervasive::MonadicPervasive;

use crate::utils::OptExt;

macro_rules! bijective_struct {
    (
        $(#[derive($($derives: ident),*)])?
        $vis: vis enum $name: ident {
            $($variant: ident => $var_name: literal),*
            $(,)?
        }
    ) =>{
        $(#[derive($($derives),*)])?
        $vis enum $name {
            $($variant),*
        }

        impl $name {
            pub const fn as_str(&self) -> &'static [u8] {
                match self {
                    $(Self::$variant => $var_name),*
                }
            }

            pub fn from_str(str: &[u8]) -> Option<Self> {
                match str {
                    $($var_name => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

bijective_struct! {
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ListStat {
    Mean => b"mean",
    Min => b"min",
    Max => b"max",
    Total => b"total",
}}

bijective_struct! {
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MonadicNonPervasive {
    Length => b"length",
    Unique => b"unique",
}}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Builtins {
    MonadicPervasive(MonadicPervasive),
    DyadicPervasive(DyadicPervasive),
    MonadicNonPervasive(MonadicNonPervasive),
    ListStat(ListStat),

    Join,   // variadic non-pervasive
    Sort,   // monadic/dyadic non-pervasive
    Random, // zero-adic / monadic non-pervasive / dyadic non-pervasive
}

macro_rules! try_options {
    (;maps: $($expr: expr => $func: expr;)* ;direct: $($simple_expr: expr => $value: expr;)*) => {
        $(if let Some(x) = $expr { Some($func(x)) } else)*
        $(if $simple_expr { Some($value) } else)*
        { None }
    };
}

impl Builtins {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MonadicPervasive(x) => x.as_str(),
            Self::DyadicPervasive(x) => x.as_str(),
            Self::MonadicNonPervasive(x) => unsafe { std::str::from_utf8_unchecked(x.as_str()) },
            Self::ListStat(x) => unsafe { std::str::from_utf8_unchecked(x.as_str()) },

            Self::Join => "join",
            Self::Sort => "sort",
            Self::Random => "random",
        }
    }

    pub fn from_str(input: &[u8]) -> Option<Self> {
        try_options! {
            ;maps:
                MonadicPervasive::from_str(input) => Self::MonadicPervasive;
                DyadicPervasive::from_str(input) => Self::DyadicPervasive;
                MonadicNonPervasive::from_str(input) => Self::MonadicNonPervasive;
                ListStat::from_str(input) => Self::ListStat;
            ;direct:
                input == b"join" => Self::Join;
                input == b"sort" => Self::Sort;
                input == b"random" => Self::Random;
        }
    }
}
