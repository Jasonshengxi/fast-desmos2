use crate::executor::evaluator::IdentId;
use crate::lexing::{Builtins, Element, Span, Token, TokenKind};
use bitflags::bitflags;
use color_eyre::owo_colors::OwoColorize;
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};

pub struct AstNode {
    span: Span,
    kind: Box<AstKind>,
}

impl Debug for AstNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstNode")
            .field("span", &self.span)
            .field("kind", &self.kind)
            .finish()
    }
}

struct DebugWrapper<T>(T);
impl<T: Debug> Display for DebugWrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AstNode {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn kind(&self) -> &AstKind {
        &self.kind
    }

    pub fn span_as_str<'a>(&self, source: &'a str) -> &'a str {
        self.span.select(source)
    }

    pub fn display(&self, source: &str, indent: usize) {
        self.display_indented(source, indent, '#')
    }

    fn display_indented(&self, source: &str, ind: usize, bar: char) {
        const T_BAR: char = '#';
        let indent = format!("{} ", '│').repeat(ind);
        if ind > 0 {
            print!(
                "{}{} ",
                (&indent[('│'.len_utf8() + 1)..]).white(),
                bar.bright_white()
            );
        }
        println!("{}{}", "span: ".green(), self.span_as_str(source));
        let indent = indent.white();
        print!("{indent}{}", "kind: ".green());
        match Box::as_ref(&self.kind) {
            AstKind::Identifier(item) => {
                println!("{}", "Identifier".bright_red());
                if let Some(item) = item.get() {
                    println!("{}{}", "Id: ".green(), item.blue());
                }
            }
            AstKind::Builtins(builtins) => {
                println!(
                    "{}{:?}{}",
                    "Builtins(".bright_red(),
                    builtins.bright_red(),
                    ")".bright_red(),
                );
            }
            AstKind::Number(num) => println!(
                "{}{}{}",
                "Number(".bright_red(),
                num.blue(),
                ")".bright_red()
            ),
            AstKind::Group(item) => {
                println!("{}", "Group".bright_red());
                item.display_indented(source, ind + 1, T_BAR);
            }
            AstKind::LatexGroup(item) => {
                println!("{}", "LatexGroup".bright_red());
                item.display_indented(source, ind + 1, T_BAR);
            }
            AstKind::Abs(item) => {
                println!("{}", "Abs".bright_red());
                item.display_indented(source, ind + 1, T_BAR);
            }
            AstKind::Point(x, y) => {
                println!("{}", "Point".bright_red());
                x.display_indented(source, ind + 1, 'x');
                y.display_indented(source, ind + 1, 'y');
            }
            AstKind::List(items) => {
                println!("{}", "List".bright_red());
                for item in items {
                    item.display_indented(source, ind + 1, 'I');
                }
            }
            AstKind::SumProd {
                kind,
                from,
                to,
                expr,
            } => {
                match kind {
                    SumOrProd::Sum => println!("{}", "Sum".bright_red()),
                    SumOrProd::Prod => println!("{}", "Prod".bright_red()),
                }
                from.display_indented(source, ind + 1, 'F');
                to.display_indented(source, ind + 1, 'T');
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::AddSub(items) => {
                println!("{}", "AddSub".bright_red());
                for (kind, item) in items {
                    item.display_indented(
                        source,
                        ind + 1,
                        match kind {
                            AddOrSub::Add => '+',
                            AddOrSub::Sub => '-',
                        },
                    );
                }
            }
            AstKind::Multiply(items) => {
                println!("{}", "Multiply".bright_red());
                for item in items {
                    item.display_indented(source, ind + 1, '*');
                }
            }
            AstKind::FunctionCall {
                ident,
                power,
                params,
            } => {
                println!("{}", "FunctionCall".bright_red());
                println!("{indent}{}{}", "Ident: ".green(), ident.span_as_str(source));
                if let Some(pow) = power {
                    pow.display_indented(source, ind + 1, '^');
                }
                for item in params {
                    item.display_indented(source, ind + 1, 'P');
                }
            }
            AstKind::Frac { above, below } => {
                println!("{}", "Frac".bright_red());
                above.display_indented(source, ind + 1, 'N');
                below.display_indented(source, ind + 1, 'D');
            }
            AstKind::Root { root, expr } => {
                println!("{}", "Root".bright_red());
                if let Some(root) = root.as_ref() {
                    root.display_indented(source, ind + 1, 'R')
                }
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::Exp { exp, expr } => {
                println!("{}", "Exp".bright_red());
                exp.display_indented(source, ind + 1, '^');
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::ListComp { expr, defs } => {
                println!("{}", "ListComp".bright_red());
                for def in defs {
                    def.display_indented(source, ind + 1, 'D');
                }
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::For { expr, defs } => {
                println!("{}", "For".bright_red());
                for def in defs {
                    def.display_indented(source, ind + 1, 'D');
                }
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::ListRange { from, next, to } => {
                println!("{}", "ListRange".bright_red());
                from.display_indented(source, ind + 1, 'F');
                if let Some(next) = next.as_ref() {
                    next.display_indented(source, ind + 1, 'N');
                }
                to.display_indented(source, ind + 1, 'T');
            }
            AstKind::Conditional { exprs, comps } => {
                println!("{}", "Conditional".bright_red());
                let mut exprs = exprs.iter();
                if let Some(expr) = exprs.next() {
                    expr.display_indented(source, ind + 1, '#');
                    for (expr, comp) in exprs.zip(comps) {
                        expr.display_indented(source, ind + 1, comp.reference_char());
                    }
                }
            }
            AstKind::IfElse { conds, yes, no } => {
                println!("{}", "IfElse".bright_red());
                for cond in conds {
                    cond.display_indented(source, ind + 1, 'C');
                }
                if let Some(yes) = yes {
                    yes.display_indented(source, ind + 1, 'Y');
                }
                if let Some(no) = no {
                    no.display_indented(source, ind + 1, 'N');
                }
            }
            AstKind::Definition { .. } => {}
            AstKind::VarDef { ident, expr } => {
                println!("{}", "VarDef".bright_red());
                ident.display_indented(source, ind + 1, 'I');
                expr.display_indented(source, ind + 1, 'E');
            }
            AstKind::ElemAccess { expr, element } => {
                println!("{}", "ElemAccess".bright_red());
                match element {
                    Element::X => println!("{}", "ElementX".red()),
                    Element::Y => println!("{}", "ElementY".red()),
                }
                expr.display_indented(source, ind + 1, 'E')
            }
            AstKind::ListIndexing { expr, index } => {
                println!("{}", "ListIndexing".bright_red());
                expr.display_indented(source, ind + 1, 'E');
                index.display_indented(source, ind + 1, 'I');
            }
            AstKind::With {
                def: substitute,
                expr,
            } => {
                println!("{}", "With".bright_red());
                substitute.display_indented(source, ind + 1, 'S');
                expr.display_indented(source, ind + 1, 'E');
            }
        }
    }
}

impl AstNode {
    pub fn update_self_span(mut self, x: Span) -> Self {
        self.span = self.span.union(x);
        self
    }

    pub fn new(span: Span, kind: AstKind) -> Self {
        Self {
            span,
            kind: Box::new(kind),
        }
    }

    pub fn try_map_token<E>(
        token: Token,
        func: impl FnOnce(TokenKind) -> Result<AstKind, E>,
    ) -> Result<Self, E> {
        Ok(Self {
            span: token.span,
            kind: Box::new(func(token.kind)?),
        })
    }

    pub fn point((lp, x, _, y, rp): (Token, Self, Token, Self, Token)) -> Self {
        let span = Span::union_n([lp.span, x.span, y.span, rp.span]);
        Self::new(span, AstKind::Point(x, y))
    }

    pub fn paren_group((lp, x, rp): (Token, Self, Token)) -> Self {
        let span = Span::union_n([lp.span, x.span, rp.span]);
        Self::new(span, AstKind::Group(x))
    }

    pub fn latex_group((lp, x, rp): (Token, Self, Token)) -> Self {
        let span = Span::union_n([lp.span, x.span, rp.span]);
        Self::new(span, AstKind::LatexGroup(x))
    }

    pub fn group_abs((lp, x, rp): (Token, Self, Token)) -> Self {
        let span = Span::union_n([lp.span, x.span, rp.span]);
        Self::new(span, AstKind::Abs(x))
    }

    pub fn list_literal((lb, items, rb): (Token, Vec<AstNode>, Token)) -> Self {
        let span = Span::union(lb.span, rb.span);

        match <[AstNode; 1]>::try_from(items) {
            Ok([one_item]) => match *one_item.kind {
                AstKind::For { expr, defs } => Self::new(span, AstKind::ListComp { expr, defs }),
                _ => Self::new(span, AstKind::List(vec![one_item])),
            },
            Err(items) => Self::new(span, AstKind::List(items)),
        }
    }

    pub fn mult(items: Vec<AstNode>) -> Self {
        let span = Span::union_n(items.iter().map(|x| x.span));
        Self::new(span, AstKind::Multiply(items))
    }

    pub fn add_sub(extra_span: Option<Span>, items: Vec<(AddOrSub, AstNode)>) -> Self {
        let span = Span::union_n(items.iter().map(|x| x.1.span));
        Self::new(
            match extra_span {
                None => span,
                Some(x) => span.union(x),
            },
            AstKind::AddSub(items),
        )
    }

    pub fn frac((frac, above, below): (Token, AstNode, AstNode)) -> Self {
        Self::new(
            Span::union_n([frac.span, above.span, below.span]),
            AstKind::Frac { above, below },
        )
    }

    pub fn root((sqrt, root, expr): (Token, Option<AstNode>, AstNode)) -> Self {
        Self::new(
            Span::union_n(
                [sqrt.span, expr.span]
                    .into_iter()
                    .chain(root.iter().map(|x| x.span)),
            ),
            AstKind::Root { root, expr },
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SumOrProd {
    Sum,
    Prod,
}

#[derive(Debug)]
pub enum AstKind {
    Identifier(Cell<Option<IdentId>>),
    Builtins(Builtins),
    Number(f64),
    Group(AstNode),
    LatexGroup(AstNode),
    Abs(AstNode),
    Point(AstNode, AstNode),
    List(Vec<AstNode>),
    VarDef {
        ident: AstNode,
        expr: AstNode,
    },
    SumProd {
        kind: SumOrProd,
        from: AstNode,
        to: AstNode,
        expr: AstNode,
    },
    FunctionCall {
        ident: AstNode,
        power: Option<AstNode>,
        params: Vec<AstNode>,
    },
    Multiply(Vec<AstNode>),
    AddSub(Vec<(AddOrSub, AstNode)>),
    Frac {
        above: AstNode,
        below: AstNode,
    },
    Root {
        root: Option<AstNode>,
        expr: AstNode,
    },
    Exp {
        expr: AstNode,
        exp: AstNode,
    },
    For {
        expr: AstNode,
        defs: Vec<AstNode>,
    },
    ListComp {
        expr: AstNode,
        defs: Vec<AstNode>,
    },
    ListRange {
        from: AstNode,
        next: Option<AstNode>,
        to: AstNode,
    },
    Conditional {
        exprs: Vec<AstNode>,
        comps: Vec<CompSet>,
    },
    IfElse {
        conds: Vec<AstNode>,
        yes: Option<AstNode>,
        no: Option<AstNode>,
    },
    Definition {
        ident: AstNode,
        expr: AstNode,
    },
    ElemAccess {
        expr: AstNode,
        element: Element,
    },
    ListIndexing {
        expr: AstNode,
        index: AstNode,
    },
    With {
        def: AstNode,
        expr: AstNode,
    },
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct CompSet: u8 {
        const MORE = 0b100;
        const EQUAL = 0b010;
        const LESS = 0b001;
        const LESS_OR_EQUAL = 0b011;
        const MORE_OR_EQUAL = 0b110;
    }
}

impl From<Ordering> for CompSet {
    fn from(value: Ordering) -> Self {
        Self::from_ordering(value)
    }
}

impl CompSet {
    pub const fn from_ordering(ordering: Ordering) -> Self {
        match ordering {
            Ordering::Less => Self::LESS,
            Ordering::Equal => Self::EQUAL,
            Ordering::Greater => Self::MORE,
        }
    }

    pub const fn reference_char(self) -> char {
        const EQUAL: u8 = CompSet::EQUAL.bits();
        const MORE: u8 = CompSet::MORE.bits();
        const LESS: u8 = CompSet::LESS.bits();
        const MORE_OR_EQ: u8 = CompSet::MORE_OR_EQUAL.bits();
        const LESS_OR_EQ: u8 = CompSet::LESS_OR_EQUAL.bits();

        match self.bits() {
            EQUAL => '=',
            MORE => '>',
            LESS => '<',
            MORE_OR_EQ => 'G',
            LESS_OR_EQ => 'L',
            _ => '#',
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum AddOrSub {
    #[default]
    Add,
    Sub,
}
