use crate::lexing::{Builtins, Span};
use crate::parsing::SumOrProd;
use crate::utils::OptExt;
use color_eyre::owo_colors::OwoColorize;
use fast_desmos2_comms::value::ops::CrossIterError;
use fast_desmos2_comms::value::ValueKind;
use glam::DVec2;
use std::mem::MaybeUninit;
use std::fmt::Display;

use crate::{
    parsing::{AddOrSub, AstKind, AstNode},
    utils::{IdVec, SparseVec},
};
pub use error::{EvalError, EvalErrorKind};
use fast_desmos2_comms::{value, List as ValueList, TypeMismatch, Value};
use funcs::*;

mod error;
mod funcs;

pub fn main() -> color_eyre::Result<()> {
    use crate::parsing;
    use color_eyre::eyre::eyre;

    let source = r"\length([1,2,3,4,4,4,4])";
    let parsed = parsing::parse_source(source)?;
    let ast = parsed.borrow_dependent().as_ref().map_err(|parse_err| {
        if let Some(parse_err) = parse_err {
            parsing::print_tree_err(source, parse_err, 0);
        }
        eyre!("Parse failed!")
    })?;

    // ast.display(source, 0);
    print!("{}", "INPUT: ".bold().green());
    let tokens = parsed.borrow_owner();
    let mut alternate = false;
    for token in tokens {
        alternate = !alternate;
        match alternate {
            true => print!("{}", token.span(source).blue()),
            false => print!("{}", token.span(source).yellow()),
        }
    }
    println!();

    let vars = OwnedVars::new();
    let idents = Identifiers::new();
    let funcs = FuncStorer::default();
    let mut evaluator = Evaluator::new(LayeredVars::base(&vars), &idents, &funcs);
    let result = evaluator.evaluate(source, ast);
    // let idents = evaluator.into_idents();
    match result {
        Ok(item) => item.display(),
        Err(err) => err.display(source),
    }

    Ok(())
}

#[derive(Default)]
pub struct Identifiers {
    ids: IdVec<String>,
}

impl Identifiers {
    pub fn new() -> Self {
        Self { ids: IdVec::new() }
    }

    pub fn to_id(&self, ident: &str) -> IdentId {
        IdentId(self.ids.id_or_insert_with(ident, || ident.to_string()))
    }

    pub fn get_name(&self, id: IdentId) -> &str {
        &self.ids[id.0]
    }
}

pub struct Function {
    params: Vec<IdentId>,
    expr: AstNode,
}

#[derive(Default)]
pub struct FuncStorer {
    funcs: SparseVec<Function>,
}

impl FuncStorer {
    pub fn get_function(&self, id: IdentId) -> Option<&Function> {
        self.funcs.get(id.0)
    }

    pub fn insert_function(&mut self, id: IdentId, expr: Function) {
        self.funcs.insert(id.0, expr);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IdentId(usize);

impl Display for IdentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type EvalResult<T> = Result<T, EvalError>;

#[derive(Default)]
pub struct OwnedVars {
    vars: SparseVec<Value>,
}

impl OwnedVars {
    fn get_value(&self, id: IdentId) -> Option<&Value> {
        self.vars.get(id.0)
    }

    fn insert_value(&mut self, id: IdentId, value: Value) {
        self.vars.insert(id.0, value);
    }
}

impl OwnedVars {
    pub fn new() -> Self {
        Self::default()
    }
}

pub enum VarLayer {
    One { id: IdentId, value: Value },
    Many(OwnedVars),
}

impl VarLayer {
    fn get_value(&self, id: IdentId) -> Option<&Value> {
        match self {
            Self::Many(x) => x.get_value(id),
            Self::One { id: this_id, value } => (*this_id == id).then_some(value),
        }
    }

    fn insert_value(&mut self, id: IdentId, value: Value) {
        match self {
            Self::Many(x) => x.insert_value(id, value),
            Self::One {
                id: this_id,
                value: this_value,
            } => {
                assert_eq!(id, *this_id, "can only set the one var for a `OneVar`");
                *this_value = value;
            }
        }
    }

    fn one(id: IdentId, value: Value) -> Self {
        Self::One { id, value }
    }
}

pub enum LayeredVars<'a> {
    Base(&'a OwnedVars),
    Layered { base: &'a Self, layer: VarLayer },
}

impl<'a> LayeredVars<'a> {
    pub fn add_layer(&'a self, layer: VarLayer) -> Self {
        Self::Layered { base: self, layer }
    }
}

impl<'a> LayeredVars<'a> {
    pub fn base(base: &'a OwnedVars) -> Self {
        Self::Base(base)
    }

    fn get_value(&self, id: IdentId) -> Option<&Value> {
        match self {
            Self::Base(vars) => vars.get_value(id),
            Self::Layered { base, layer } => {
                layer.get_value(id).map_or_else(|| base.get_value(id), Some)
            }
        }
    }

    fn insert_value(&mut self, id: IdentId, value: Value) {
        match self {
            Self::Base(_) => unreachable!("cannot modify base layer"),
            Self::Layered { base: _, layer } => layer.insert_value(id, value),
        }
    }
}

pub struct Evaluator<'a> {
    vars: LayeredVars<'a>,
    idents: &'a Identifiers,
    funcs: &'a FuncStorer,
}

impl<'a> Evaluator<'a> {
    pub fn into_vars(self) -> LayeredVars<'a> {
        self.vars
    }

    pub fn new(vars: LayeredVars<'a>, idents: &'a Identifiers, funcs: &'a FuncStorer) -> Self {
        Self {
            vars,
            idents,
            funcs,
        }
    }

    fn derived_evaluator(&self, vars: LayeredVars<'a>) -> Self {
        Evaluator {
            vars,
            idents: self.idents,
            funcs: self.funcs,
        }
    }

    pub fn do_identifier(&mut self, span: &str, value: &IdentCell) -> IdentId {
        match value.get() {
            None => {
                let ident = self.idents.to_id(span);
                value.set(Some(ident));
                ident
            }
            Some(id) => id,
        }
    }

    pub fn evaluate(&mut self, source: &str, item: &AstNode) -> EvalResult<Value> {
        use EvalErrorKind as EKind;

        let node_span = item.span();
        match item.kind() {
            AstKind::VarDef { .. } => unreachable!(),
            AstKind::Identifier(value) => {
                let ident = self.do_identifier(item.span_as_str(source), value);

                self.vars
                    .get_value(ident)
                    .ok_or(EvalErrorKind::UnknownIdent(ident).with_span(node_span))
                    .cloned()
            }
            AstKind::Group(expr) | AstKind::LatexGroup(expr) => self.evaluate(source, expr),
            &AstKind::Number(num) => Ok(Value::Number(ValueList::Term(num))),
            AstKind::List(nodes) => nodes
                .iter()
                .try_fold(
                    (Span::EMPTY, Value::Number(ValueList::empty())),
                    |(span, mut list), item| -> EvalResult<(Span, Value)> {
                        let item_val = self.evaluate(source, item)?;
                        let result = list.push(item_val).map(|_| list);
                        maybe_type_mismatch(result, span, item.span(), "list literal")
                    },
                )
                .map(|x| x.1),
            AstKind::AddSub(pairs) => pairs
                .iter()
                .try_fold(
                    (Span::EMPTY, None),
                    |(old_span, sum), (add_sub, item)| -> EvalResult<(Span, Option<Value>)> {
                        let value = self.evaluate(source, item)?;

                        if let Some(sum) = sum {
                            let (result, op_name) = match add_sub {
                                AddOrSub::Add => (sum + value, "addition"),
                                AddOrSub::Sub => (sum - value, "subtraction"),
                            };
                            maybe_type_mismatch(result, old_span, item.span(), op_name)
                                .map(|(a, b)| (a, Some(b)))
                        } else {
                            let value = match add_sub {
                                AddOrSub::Add => value,
                                AddOrSub::Sub => (-value).ok_or_else(|| {
                                    EKind::InvalidValue {
                                        for_what: "negation",
                                    }
                                    .with_span(item.span())
                                })?,
                            };
                            Ok((item.span(), Some(value)))
                        }
                    },
                )
                .map(|x| x.1.unwrap_unreach()),
            AstKind::Frac { above, below } => {
                let above_val = self.evaluate(source, above)?;
                let below_val = self.evaluate(source, below)?;
                maybe_type_mismatch(
                    above_val / below_val,
                    above.span(),
                    below.span(),
                    "division",
                )
                .map(|x| x.1)
            }
            AstKind::Multiply(nodes) => nodes
                .iter()
                .try_fold(
                    (Span::EMPTY, None),
                    |(old_span, acc), node| -> EvalResult<(Span, Option<Value>)> {
                        let value = self.evaluate(source, node)?;

                        if let Some(acc) = acc {
                            maybe_type_mismatch(
                                acc * value,
                                old_span,
                                node.span(),
                                "multiplication",
                            )
                            .map(|(a, b)| (a, Some(b)))
                        } else {
                            Ok((node.span(), Some(value)))
                        }
                    },
                )
                .map(|x| x.1.unwrap_unreach()),
            AstKind::Point(x, y) => {
                let x_val = self
                    .evaluate(source, x)?
                    .try_number()
                    .map_err(|m| wrong_type(m, x.span()))?;
                let y_val = self
                    .evaluate(source, y)?
                    .try_number()
                    .map_err(|m| wrong_type(m, y.span()))?;

                let result = value::ops::iter_full(x_val, y_val, &|x, y| DVec2 { x, y });

                Ok(Value::Point(result))
            }
            AstKind::Root { root, expr } => {
                let root_value = root
                    .as_ref()
                    .map(|root| self.evaluate(source, root))
                    .transpose()
                    .map(|x| x.unwrap_or(Value::Number(ValueList::Term(2.0))))?;

                let Value::Number(root_num) = root_value else {
                    return Err(EKind::WrongType {
                        expect: ValueKind::Number,
                        got: root_value.kind(),
                    }
                    .with_span(root.as_ref().unwrap_unreach().span()));
                };

                let ValueList::Term(root_num) = root_num else {
                    return Err(EKind::InvalidValue {
                        for_what: "nth root",
                    }
                    .with_span(root.as_ref().unwrap_unreach().span())
                    .with_note("nth root operator expects one number on the exponent"));
                };

                let expr_value = self.evaluate(source, expr)?;
                let Value::Number(numbers) = expr_value else {
                    return Err(EKind::WrongType {
                        expect: ValueKind::Number,
                        got: expr_value.kind(),
                    }
                    .with_span(expr.span())
                    .with_note("nth root can only be taken on numbers"));
                };

                Ok(Value::Number(value::ops::iter_alone_left(
                    numbers,
                    root_num.recip(),
                    &|x, exp| x.powf(exp),
                )))
            }
            AstKind::ListRange { from, next, to } => {
                let from_val = self
                    .evaluate(source, from)?
                    .try_number()
                    .map_err(|m| wrong_type(m, from.span()))?;
                let to_val = self
                    .evaluate(source, to)?
                    .try_number()
                    .map_err(|m| wrong_type(m, to.span()))?;

                let step = match next.as_ref() {
                    None => ValueList::Term(1.0),
                    Some(node) => {
                        self.evaluate(source, node)?
                            .try_number()
                            .map_err(|m| wrong_type(m, node.span()))?
                            - from_val.clone()
                    }
                };

                let step1 = value::ops::iter_full(from_val, step, &|x, y| (x, y));
                let result = value::ops::iter_full(step1, to_val, &|(from, step), to| {
                    let mut result = Vec::new();
                    let mut counter = from;
                    while counter <= to {
                        result.push(counter);
                        counter += step;
                    }
                    result
                });

                fn flatten_vec_list<T>(input: ValueList<Vec<T>>) -> ValueList<T> {
                    match input {
                        ValueList::Term(x) => ValueList::Flat(x),
                        ValueList::Flat(xs) => {
                            ValueList::Staggered(xs.into_iter().map(ValueList::Flat).collect())
                        }
                        ValueList::Staggered(xs) => {
                            ValueList::Staggered(xs.into_iter().map(flatten_vec_list).collect())
                        }
                    }
                }

                Ok(Value::Number(flatten_vec_list(result)))
            }
            AstKind::Exp { expr, exp } => {
                let expr_val = self
                    .evaluate(source, expr)?
                    .try_number()
                    .map_err(|m| wrong_type(m, expr.span()))?;
                let exp_val = self
                    .evaluate(source, exp)?
                    .try_number()
                    .map_err(|m| wrong_type(m, exp.span()))?;

                Ok(Value::Number(value::ops::iter_full(
                    expr_val,
                    exp_val,
                    &|a, b| a.powf(b),
                )))
            }
            AstKind::SumProd {
                kind,
                from,
                to,
                expr,
            } => {
                let (id_str, id_cell, item) = unwrap_var_def(source, from);
                let id = self.do_identifier(id_str, id_cell);

                let from_val = self
                    .evaluate(source, item)?
                    .try_number()
                    .map_err(|m| wrong_type(m, from.span()))?;

                let to_val = self
                    .evaluate(source, to)?
                    .try_number()
                    .map_err(|m| wrong_type(m, to.span()))?;

                #[rustfmt::skip]
                let result = value::ops::try_iter_full(
                    from_val,
                    to_val,
                    &|from, to| -> EvalResult<Value> {
                        let mut counter = from;
                        let mut result = None;

                        while counter <= to {
                            let layer = VarLayer::one(id, Value::Number(ValueList::Term(counter)));
                            let new_vars = self.vars.add_layer(layer);
                            let mut new_evaluator = self.derived_evaluator(new_vars);

                            let new_value = new_evaluator.evaluate(source, expr)?;

                            result = Some(match result {
                                None => new_value,
                                Some(result) => {
                                    let sum: Result<Value, _> = match kind {
                                        SumOrProd::Sum => result + new_value,
                                        SumOrProd::Prod => result * new_value,
                                    };
                                    sum.map_err(|m| wrong_type(m, expr.span()))?
                                }
                            });

                            counter += 1.0;
                        }

                        Ok(result.unwrap_or_else(|| todo!("Oh god please no")))
                    }
                )?;

                result
                    .flatten_value()
                    .map_err(|m| wrong_type(m, expr.span()))
            }
            AstKind::FunctionCall {
                ident,
                power,
                params,
            } => {
                let power = if let Some(node) = power {
                    self.evaluate(source, node)?.try_number().map_err(|m| {
                        wrong_type(m, node.span()).with_note("function call expects numeric power")
                    })?
                } else {
                    ValueList::Term(1.0)
                };
                match ident.kind() {
                    AstKind::Identifier(id_cell) => {
                        let id = self.do_identifier(ident.span_as_str(source), id_cell);

                        let func = self
                            .funcs
                            .get_function(id)
                            .ok_or_else(|| EKind::UnknownIdent(id).with_span(ident.span()))?;

                        if params.len() != func.params.len() {
                            return Err(EKind::BadParamCount {
                                expect: func.params.len(),
                                got: params.len(),
                            }
                            .with_span(node_span));
                        }

                        let mut new_vars = OwnedVars::new();
                        for (id, expr) in func.params.iter().copied().zip(params) {
                            let value = self.evaluate(source, expr)?;
                            new_vars.insert_value(id, value);
                        }
                        let new_vars = self.vars.add_layer(VarLayer::Many(new_vars));

                        self.derived_evaluator(new_vars)
                            .evaluate(source, &func.expr)
                    }
                    &AstKind::Builtins(builtins) => match builtins {
                        Builtins::MonadicPervasive(monadic) => {
                            if params.len() != 1 {
                                return Err(EKind::BadParamCount {
                                    expect: 1,
                                    got: params.len(),
                                }
                                .with_span(node_span));
                            };
                            let param = &params[0];
                            let value = self
                                .evaluate(source, param)?
                                .try_number()
                                .map_err(|m| wrong_type(m, param.span()))?;
                            Ok(Value::Number(value::ops::iter_full(
                                power,
                                value,
                                &|pow, val| -> f64 {
                                    let (mon, p) = match pow {
                                        -1.0 => monadic
                                            .invert()
                                            .map_or((monadic, Some(-1.0)), |mon| (mon, None)),
                                        _ => (monadic, Some(pow)),
                                    };

                                    let val = mon.apply_one(val);
                                    match p {
                                        None => val,
                                        Some(x) => val.powf(x),
                                    }
                                },
                            )))
                        }
                        Builtins::MonadicNonPervasive(mon) => {
                            if params.len() != 1 {
                                return Err(EKind::BadParamCount {
                                    expect: 1,
                                    got: params.len(),
                                }
                                .with_span(node_span));
                            };
                            let param = &params[0];
                            let value = self.evaluate(source, param)?;
                            Ok(mon.apply(value))
                        }
                        Builtins::DyadicPervasive(dyadic) => {
                            if params.len() != 2 {
                                return Err(EKind::BadParamCount {
                                    expect: 2,
                                    got: params.len(),
                                }
                                .with_span(node_span));
                            }
                            let left = &params[0];
                            let right = &params[1];
                            let left_val = self.evaluate(source, left)?;
                            let right_val = self.evaluate(source, right)?;
                            dyadic
                                .type_check(
                                    (left.span(), left_val.kind()),
                                    (right.span(), right_val.kind()),
                                )
                                .map_err(|ek| ek.with_span(node_span))?;

                            value::ops::try_iter_many_known(
                                [left_val.as_ref(), right_val.as_ref()],
                                &mut |[l, r]| {
                                    dyadic.apply_one(l, r).map_err(|ek| ek.with_span(node_span))
                                },
                                &|err| -> EvalError { EKind::wrong_type(err).with_span(node_span) },
                            )
                        }
                        _ => todo!(),
                    },
                    _ => unreachable!(),
                }
            }
            AstKind::With { def, expr } => {
                let (id_str, id_cell, value_node) = unwrap_var_def(source, def);
                let value = self.evaluate(source, value_node)?;
                let id = self.do_identifier(id_str, id_cell);
                let new_vars = self.vars.add_layer(VarLayer::One { id, value });

                self.derived_evaluator(new_vars).evaluate(source, expr)
            }
            AstKind::For { expr, defs } | AstKind::ListComp { expr, defs } => {
                let (ids, exprs): (Vec<_>, Vec<_>) = defs
                    .iter()
                    .map(|def| {
                        let (name, id_cell, expr) = unwrap_var_def(source, def);
                        let id = self.do_identifier(name, id_cell);
                        (id, expr)
                    })
                    .unzip();

                let values: Vec<_> = exprs
                    .into_iter()
                    .map(|expr| self.evaluate(source, expr))
                    .collect::<EvalResult<_>>()?;

                let mut new_vars = self.vars.add_layer(VarLayer::Many(OwnedVars::new()));
                value::ops::try_cross_iter_many(
                    values.iter().map(Value::as_ref).collect(),
                    &mut |values| {
                        for (&id, one_ref) in ids.iter().zip(values) {
                            new_vars.insert_value(id, one_ref.to_value());
                        }

                        let mut result = MaybeUninit::uninit();
                        take_mut::take(&mut new_vars, |n_vars| {
                            let mut evaluator = self.derived_evaluator(n_vars);
                            result.write(evaluator.evaluate(source, expr));
                            evaluator.into_vars()
                        });
                        unsafe { result.assume_init() }
                    },
                    &|err| -> EvalError {
                        match err {
                            CrossIterError::TypeMismatch(mismatch) => EKind::TypeMismatch {
                                op_name: "`for` operator",
                                left: (expr.span(), mismatch.expect),
                                right: (expr.span(), mismatch.got),
                            }
                            .with_span(node_span),
                            CrossIterError::TooLong => EKind::InvalidValue {
                                for_what: "`for` operator",
                            }
                            .with_span(node_span)
                            .with_note("the resulting list was too long"),
                        }
                    },
                )
            }
            kind => todo!("Implement evaluation for {kind:?}"),
        }
    }
}
