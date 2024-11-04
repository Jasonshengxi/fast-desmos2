use fast_desmos2_comms::{TypeMismatch, Value};

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
pub enum MonadicPervasive {
    Sin => b"sin",
    Cos => b"cos",
    Tan => b"tan",
    Sec => b"sec",
    Csc => b"csc",
    Cot => b"cot",

    Sinh => b"sinh",
    Cosh => b"cosh",
    Tanh => b"tanh",
    Sech => b"sech",
    Csch => b"csch",
    Coth => b"coth",

    ArcSin => b"arcsin",
    ArcCos => b"arccos",
    ArcTan => b"arctan",
    ArcSec => b"arcsec",
    ArcCsc => b"arccsc",
    ArcCot => b"arccot",

    ArcSinh => b"arcsinh",
    ArcCosh => b"arccosh",
    ArcTanh => b"arctanh",
    ArcSech => b"arcsech",
    ArcCsch => b"arccsch",
    ArcCoth => b"arccoth",

    Sign => b"sign",
    Floor => b"floor",
    Ceil => b"ceil",
    Round => b"round",
}}

bijective_struct! {
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum DyadicPervasive {
    Mod => b"mod",
    Choose => b"choose",
    Permutation => b"permutation",
    Distance => b"distance",
}}

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

impl MonadicPervasive {
    pub fn apply(&self, target: Value) -> Result<Value, TypeMismatch> {
        target
            .try_number()
            .map(|numbers| Value::Number(numbers.map(&f64::sin)))
    }
}
