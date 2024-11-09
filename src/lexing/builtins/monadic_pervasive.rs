use std::str::FromStr;

use fast_desmos2_comms::List as ValueList;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MonadicPervasive {
    Sin,
    Cos,
    Tan,
    Sec,
    Csc,
    Cot,

    Sinh,
    Cosh,
    Tanh,
    Sech,
    Csch,
    Coth,

    ArcSin,
    ArcCos,
    ArcTan,
    ArcSec,
    ArcCsc,
    ArcCot,

    ArcSinh,
    ArcCosh,
    ArcTanh,
    ArcSech,
    ArcCsch,
    ArcCoth,

    Sign,
    Floor,
    Ceil,
    Round,
}

impl FromStr for MonadicPervasive {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s.as_bytes()).ok_or(())
    }
}

impl MonadicPervasive {
    pub const fn from_str(from: &[u8]) -> Option<Self> {
        Some(match from {
            b"sin" => Self::Sin,
            b"cos" => Self::Cos,
            b"tan" => Self::Tan,
            b"sec" => Self::Sec,
            b"csc" => Self::Csc,
            b"cot" => Self::Cot,

            b"sinh" => Self::Sinh,
            b"cosh" => Self::Cosh,
            b"tanh" => Self::Tanh,
            b"sech" => Self::Sech,
            b"csch" => Self::Csch,
            b"coth" => Self::Coth,

            b"arcsin" => Self::ArcSin,
            b"arccos" => Self::ArcCos,
            b"arctan" => Self::ArcTan,
            b"arcsec" => Self::ArcSec,
            b"arccsc" => Self::ArcCsc,
            b"arccot" => Self::ArcCot,

            b"arcsinh" => Self::ArcSinh,
            b"arccosh" => Self::ArcCosh,
            b"arctanh" => Self::ArcTanh,
            b"arcsech" => Self::ArcSech,
            b"arccsch" => Self::ArcCsch,
            b"arccoth" => Self::ArcCoth,

            b"arsinh" => Self::ArcSinh,
            b"arcosh" => Self::ArcCosh,
            b"artanh" => Self::ArcTanh,
            b"arsech" => Self::ArcSech,
            b"arcsch" => Self::ArcCsch,
            b"arcoth" => Self::ArcCoth,

            b"sign" => Self::Sign,
            b"floor" => Self::Floor,
            b"ceil" => Self::Ceil,
            b"round" => Self::Round,

            _ => return None,
        })
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Sec => "sec",
            Self::Csc => "csc",
            Self::Cot => "cot",

            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Sech => "sech",
            Self::Csch => "csch",
            Self::Coth => "coth",

            Self::ArcSin => "arcsin",
            Self::ArcCos => "arccos",
            Self::ArcTan => "arctan",
            Self::ArcSec => "arcsec",
            Self::ArcCsc => "arccsc",
            Self::ArcCot => "arccot",

            Self::ArcSinh => "arcsinh",
            Self::ArcCosh => "arccosh",
            Self::ArcTanh => "arctanh",
            Self::ArcSech => "arcsech",
            Self::ArcCsch => "arccsch",
            Self::ArcCoth => "arccoth",

            Self::Sign => "sign",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
        }
    }

    pub fn invert(self) -> Option<Self> {
        Some(match self {
            Self::Sin => Self::ArcSin,
            Self::Cos => Self::ArcCos,
            Self::Tan => Self::ArcTan,
            Self::Sec => Self::ArcSec,
            Self::Csc => Self::ArcCsc,
            Self::Cot => Self::ArcCot,

            Self::Sinh => Self::ArcSinh,
            Self::Cosh => Self::ArcCosh,
            Self::Tanh => Self::ArcTanh,
            Self::Sech => Self::ArcSech,
            Self::Csch => Self::ArcCsch,
            Self::Coth => Self::ArcCoth,

            Self::ArcSin => Self::Sin,
            Self::ArcCos => Self::Cos,
            Self::ArcTan => Self::Tan,
            Self::ArcSec => Self::Sec,
            Self::ArcCsc => Self::Csc,
            Self::ArcCot => Self::Cot,

            Self::ArcSinh => Self::Sinh,
            Self::ArcCosh => Self::Cosh,
            Self::ArcTanh => Self::Tanh,
            Self::ArcSech => Self::Sech,
            Self::ArcCsch => Self::Csch,
            Self::ArcCoth => Self::Coth,

            Self::Sign => return None,
            Self::Ceil => return None,
            Self::Floor => return None,
            Self::Round => return None,
        })
    }

    pub fn apply_one(&self, target: f64) -> f64 {
        match self {
            Self::Sin => f64::sin(target),
            Self::Cos => f64::cos(target),
            Self::Tan => f64::tan(target),
            Self::Sec => target.cos().recip(),
            Self::Csc => target.sin().recip(),
            Self::Cot => target.tan().recip(),

            Self::Sinh => f64::sinh(target),
            Self::Cosh => f64::cosh(target),
            Self::Tanh => f64::tanh(target),
            Self::Sech => target.cosh().recip(),
            Self::Csch => target.sinh().recip(),
            Self::Coth => target.tanh().recip(),

            Self::ArcSin => f64::asin(target),
            Self::ArcCos => f64::acos(target),
            Self::ArcTan => f64::atan(target),
            Self::ArcSec => target.recip().acos(),
            Self::ArcCsc => target.recip().asin(),
            Self::ArcCot => target.recip().atan(),

            Self::ArcSinh => f64::asinh(target),
            Self::ArcCosh => f64::acosh(target),
            Self::ArcTanh => f64::atanh(target),
            Self::ArcSech => target.recip().acosh(),
            Self::ArcCsch => target.recip().asinh(),
            Self::ArcCoth => target.recip().atanh(),

            Self::Sign => f64::signum(target),
            Self::Floor => f64::floor(target),
            Self::Ceil => f64::ceil(target),
            Self::Round => f64::round(target),
        }
    }

    pub fn apply_number(&self, numbers: ValueList<f64>) -> ValueList<f64> {
        match self {
            Self::Sin => numbers.map(&f64::sin),
            Self::Cos => numbers.map(&f64::cos),
            Self::Tan => numbers.map(&f64::tan),
            Self::Sec => numbers.map(&|x| x.cos().recip()),
            Self::Csc => numbers.map(&|x| x.sin().recip()),
            Self::Cot => numbers.map(&|x| x.tan().recip()),

            Self::Sinh => numbers.map(&f64::sinh),
            Self::Cosh => numbers.map(&f64::cosh),
            Self::Tanh => numbers.map(&f64::tanh),
            Self::Sech => numbers.map(&|x| x.cosh().recip()),
            Self::Csch => numbers.map(&|x| x.sinh().recip()),
            Self::Coth => numbers.map(&|x| x.tanh().recip()),

            Self::ArcSin => numbers.map(&f64::asin),
            Self::ArcCos => numbers.map(&f64::acos),
            Self::ArcTan => numbers.map(&f64::atan),
            Self::ArcSec => numbers.map(&|x| x.recip().acos()),
            Self::ArcCsc => numbers.map(&|x| x.recip().asin()),
            Self::ArcCot => numbers.map(&|x| x.recip().atan()),

            Self::ArcSinh => numbers.map(&f64::asinh),
            Self::ArcCosh => numbers.map(&f64::acosh),
            Self::ArcTanh => numbers.map(&f64::atanh),
            Self::ArcSech => numbers.map(&|x| x.recip().acosh()),
            Self::ArcCsch => numbers.map(&|x| x.recip().asinh()),
            Self::ArcCoth => numbers.map(&|x| x.recip().atanh()),

            Self::Sign => numbers.map(&f64::signum),
            Self::Floor => numbers.map(&f64::floor),
            Self::Ceil => numbers.map(&f64::ceil),
            Self::Round => numbers.map(&f64::round),
        }
    }
}
