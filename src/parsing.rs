use std::cell::Cell;

use crate::lexing;
use crate::lexing::{
    ContextError, LeftRight, LexError, PairedPunct, Punctuation, Span, Token, TokenKind,
};
use crate::parsing::node::CompSet;
use color_eyre::owo_colors::OwoColorize;
pub use node::{AddOrSub, AstKind, AstNode, SumOrProd};
use self_cell::self_cell;
use winnow::combinator::{
    alt, delimited, eof, opt, peek, permutation, preceded, repeat, separated, separated_pair,
    terminated,
};
use winnow::error::{
    AddContext, ErrMode, ErrorKind, ParserError, StrContext, StrContextValue, TreeError,
    TreeErrorBase, TreeErrorFrame,
};
use winnow::stream::Stream;
use winnow::token::{any, one_of};
use winnow::{PResult, Parser};
use LeftRight::{Left, Right};

mod node;
mod token;

fn punct<'a>(punct: Punctuation) -> impl Parser<Input<'a>, Token, ParseError<'a>> {
    one_of(punct).context(StrContext::Expected(StrContextValue::StringLiteral(
        punct.reference_str(),
    )))
}

fn paired<'a>(punct: PairedPunct) -> impl Parser<Input<'a>, Token, ParseError<'a>> {
    one_of(punct).context(StrContext::Expected(StrContextValue::StringLiteral(
        punct.reference_str(),
    )))
}

pub type ParsedAstNode<'a> = Parsed<'a, AstNode>;
pub type Parsed<'a, T> = PResult<T, ParseError<'a>>;
pub type ParseError<'a> = TreeError<Input<'a>>;
pub type Input<'a> = &'a [Token];

fn parse_point<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::Paren(Left)),
        parse_expr,
        punct(Punctuation::Comma),
        parse_expr,
        paired(PairedPunct::Paren(Right)),
    )
        .map(AstNode::point)
        .parse_next(input)
}

fn parse_paren_group<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::Paren(Left)),
        parse_expr,
        paired(PairedPunct::Paren(Right)),
    )
        .map(AstNode::paren_group)
        .parse_next(input)
}

fn parse_abs<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (punct(Punctuation::Abs), parse_expr, punct(Punctuation::Abs))
        .map(AstNode::group_abs)
        .parse_next(input)
}

fn parse_list_literal<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::Square(Left)),
        separated(.., parse_expr, punct(Punctuation::Comma)),
        paired(PairedPunct::Square(Right)),
    )
        .map(AstNode::list_literal)
        .parse_next(input)
}

fn parse_latex_group<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::LatexCurly(Left)),
        parse_expr,
        paired(PairedPunct::LatexCurly(Right)),
    )
        .map(AstNode::latex_group)
        .parse_next(input)
}

fn parse_identifier<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    let checkpoint = input.checkpoint();
    let result: ParsedAstNode = one_of(TokenKind::Identifier)
        .map(|token: Token| AstNode::new(token.span, AstKind::Identifier(Cell::new(None))))
        .parse_next(input);

    result.map_err(|err| {
        err.add_context(
            input,
            &checkpoint,
            StrContext::Expected(StrContextValue::Description("an identifier")),
        )
    })
}

fn parse_builtins<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    let checkpoint = input.checkpoint();
    let token = any(input)?;
    if let TokenKind::Builtins(builtins) = token.kind {
        Ok(AstNode::new(token.span, AstKind::Builtins(builtins)))
    } else {
        Err(
            ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
                input,
                &checkpoint,
                StrContext::Expected(StrContextValue::Description("a builtins")),
            ),
        )
    }
}

fn parse_number<'a>(input: &mut Input<'a>) -> Parsed<'a, f64> {
    let checkpoint = input.checkpoint();
    let token = any(input)?;
    if let TokenKind::Number(num) = token.kind {
        Ok(num)
    } else {
        Err(
            ErrMode::from_error_kind(input, ErrorKind::Verify).add_context(
                input,
                &checkpoint,
                StrContext::Expected(StrContextValue::Description("a number")),
            ),
        )
    }
}

fn parse_function_call<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        alt((parse_identifier, parse_builtins)),
        opt(preceded(
            punct(Punctuation::Exp),
            delimited(
                paired(PairedPunct::LatexCurly(Left)),
                parse_expr,
                paired(PairedPunct::LatexCurly(Right)),
            ),
        )),
        preceded(
            paired(PairedPunct::Paren(Left)),
            separated(.., parse_expr, punct(Punctuation::Comma)),
        ),
        paired(PairedPunct::Paren(Right)),
    )
        .map(
            |(ident, power, params, end): (AstNode, _, Vec<AstNode>, _)| {
                AstNode::new(
                    Span::union(
                        ident.span().union(end.span),
                        Span::union_n(params.iter().map(|node| node.span())),
                    ),
                    AstKind::FunctionCall {
                        ident,
                        power,
                        params,
                    },
                )
            },
        )
        .parse_next(input)
}

fn parse_var_def<'a>(input: &mut Input<'a>) -> Parsed<'a, AstNode> {
    separated_pair(parse_identifier, punct(Punctuation::Equals), parse_expr)
        .map(|(ident, expr)| {
            let span = Span::union(ident.span(), expr.span());
            AstNode::new(span, AstKind::VarDef { ident, expr })
        })
        .parse_next(input)
}

fn parse_sum_prod<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        alt((
            punct(Punctuation::Sum).map(|token: Token| (token.span, SumOrProd::Sum)),
            punct(Punctuation::Prod).map(|token: Token| (token.span, SumOrProd::Prod)),
        )),
        permutation((
            preceded(
                punct(Punctuation::Subscript),
                delimited(
                    paired(PairedPunct::LatexCurly(Left)),
                    parse_var_def,
                    paired(PairedPunct::LatexCurly(Right)),
                ),
            ),
            preceded(punct(Punctuation::Exp), parse_latex_group),
        )),
        parse_multiplication,
    )
        .map(|((start_span, kind), (from, to), expr)| {
            AstNode::new(
                Span::union_n([start_span, from.span(), to.span(), expr.span()]),
                AstKind::SumProd {
                    kind,
                    from,
                    to,
                    expr,
                },
            )
        })
        .parse_next(input)
}

fn parse_frac<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        punct(Punctuation::Frac),
        parse_latex_group,
        parse_latex_group,
    )
        .map(AstNode::frac)
        .parse_next(input)
}

fn parse_sqrt<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        punct(Punctuation::Sqrt),
        opt(delimited(
            paired(PairedPunct::Square(Left)),
            parse_expr,
            paired(PairedPunct::Square(Right)),
        )),
        parse_latex_group,
    )
        .map(AstNode::root)
        .parse_next(input)
}

fn parse_list_range<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::Square(Left)),
        parse_expr,
        opt(preceded(punct(Punctuation::Comma), parse_expr)),
        delimited(
            opt(punct(Punctuation::Comma)),
            punct(Punctuation::Ellipses),
            opt(punct(Punctuation::Comma)),
        ),
        parse_expr,
        paired(PairedPunct::Square(Right)),
    )
        .map(|(lp, from, next, _, to, rp)| {
            AstNode::new(
                Span::union(lp.span, rp.span),
                AstKind::ListRange { from, next, to },
            )
        })
        .parse_next(input)
}

fn parse_comparison<'a>(input: &mut Input<'a>) -> Parsed<'a, CompSet> {
    one_of([
        Punctuation::Equals,
        Punctuation::MoreThan,
        Punctuation::LessThan,
        Punctuation::MoreOrEqual,
        Punctuation::LessOrEqual,
    ])
    .map(|token: Token| {
        let TokenKind::Punct(punct) = token.kind else {
            unreachable!()
        };
        match punct {
            Punctuation::Equals => CompSet::EQUAL,
            Punctuation::MoreThan => CompSet::MORE,
            Punctuation::LessThan => CompSet::LESS,
            Punctuation::MoreOrEqual => CompSet::MORE_OR_EQUAL,
            Punctuation::LessOrEqual => CompSet::LESS_OR_EQUAL,
            _ => unreachable!(),
        }
    })
    .parse_next(input)
}

fn parse_cond<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (parse_expr, repeat(1.., (parse_comparison, parse_expr)))
        .map(|(expr, repeat): (AstNode, Vec<_>)| {
            let span = expr
                .span()
                .union(Span::union_n(repeat.iter().map(|(_, x)| x.span())));
            let mut exprs = vec![expr];
            let mut comps = Vec::with_capacity(repeat.len());
            for (comp, expr) in repeat {
                exprs.push(expr);
                comps.push(comp);
            }

            AstNode::new(span, AstKind::Conditional { exprs, comps })
        })
        .parse_next(input)
}

fn parse_if_else_contents<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        separated(1.., parse_cond, punct(Punctuation::Comma)),
        opt(preceded(
            punct(Punctuation::Colon),
            (
                parse_expr,
                opt(preceded(
                    punct(Punctuation::Comma),
                    alt((parse_if_else_contents, parse_expr)),
                )),
            ),
        )),
    )
        .map(|(conds, rem): (Vec<_>, _)| {
            let span = Span::union_n(conds.iter().map(|node| node.span()));
            let (yes, no) = rem.map_or_else(|| (None, None), |(yes, no)| (Some(yes), no));
            AstNode::new(span, AstKind::IfElse { conds, yes, no })
        })
        .parse_next(input)
}

fn parse_if_else<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        paired(PairedPunct::Curly(Left)),
        parse_if_else_contents,
        paired(PairedPunct::Curly(Right)),
    )
        .map(|(lb, if_else, rb)| if_else.update_self_span(lb.span).update_self_span(rb.span))
        .parse_next(input)
}

fn parse_everything_else<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    let token = peek(any).parse_next(input)?;

    match token.kind {
        TokenKind::Number(num) => {
            let _ = any::<_, ErrMode<ErrorKind>>(input); // consume the one peeked token
            Ok(AstNode::new(token.span, AstKind::Number(num)))
        }
        TokenKind::Builtins(_) => parse_function_call(input),
        TokenKind::Identifier => alt((parse_function_call, parse_identifier)).parse_next(input),
        TokenKind::Paired(PairedPunct::Paren(Left)) => {
            alt((parse_point, parse_paren_group)).parse_next(input)
        }
        TokenKind::Paired(PairedPunct::Square(Left)) => {
            alt((parse_list_literal, parse_list_range)).parse_next(input)
        }
        TokenKind::Paired(PairedPunct::Curly(Left)) => parse_if_else(input),
        TokenKind::Punct(Punctuation::Abs) => parse_abs(input),
        TokenKind::Punct(Punctuation::Sum | Punctuation::Prod) => parse_sum_prod(input),
        TokenKind::Punct(Punctuation::Frac) => parse_frac(input),
        TokenKind::Punct(Punctuation::Sqrt) => parse_sqrt(input),

        _ => Err(ErrMode::from_error_kind(input, ErrorKind::Alt)),
    }
}

fn parse_element<'a>(input: &mut Input<'a>) -> Parsed<'a, Token> {
    let item = peek(any).parse_next(input)?;
    if matches!(item.kind, TokenKind::Element(_)) {
        any(input)
    } else {
        Err(ErrMode::from_error_kind(input, ErrorKind::Verify))
    }
}

fn parse_postfix<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    enum Postfix {
        Elem(Token),
        Ind((AstNode, Token)),
        Exp(AstNode),
    }

    (
        parse_everything_else,
        repeat(
            ..,
            alt((
                parse_element.map(Postfix::Elem),
                preceded(
                    paired(PairedPunct::Square(Left)),
                    (parse_expr, paired(PairedPunct::Square(Right))),
                )
                .map(Postfix::Ind),
                preceded(punct(Punctuation::Exp), parse_latex_group).map(Postfix::Exp),
            )),
        ),
    )
        .map(|(expr, repeat): (_, Vec<_>)| {
            repeat
                .into_iter()
                .fold(expr, |expr, postfix| match postfix {
                    Postfix::Ind((index, rb)) => {
                        let span = Span::union(expr.span(), index.span()).union(rb.span);
                        AstNode::new(span, AstKind::ListIndexing { expr, index })
                    }
                    Postfix::Elem(elem) => {
                        let span = expr.span().union(elem.span);
                        let TokenKind::Element(element) = elem.kind else {
                            unreachable!()
                        };
                        AstNode::new(span, AstKind::ElemAccess { expr, element })
                    }
                    Postfix::Exp(exp) => {
                        let span = expr.span().union(exp.span());
                        AstNode::new(span, AstKind::Exp { expr, exp })
                    }
                })
        })
        .parse_next(input)
}

fn parse_multiplication<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    (
        parse_postfix,
        repeat(.., preceded(opt(punct(Punctuation::Times)), parse_postfix)),
    )
        .map(|(expr, mut repeat): (_, Vec<_>)| match repeat.len() {
            0 => expr,
            _ => {
                repeat.insert(0, expr);
                AstNode::mult(repeat)
            }
        })
        .parse_next(input)
}

fn parse_add_sub<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    fn plus_or_minus<'a>(input: &mut Input<'a>) -> Parsed<'a, (Span, AddOrSub)> {
        alt((
            punct(Punctuation::Plus).map(|x: Token| (x.span, AddOrSub::Add)),
            punct(Punctuation::Minus).map(|x: Token| (x.span, AddOrSub::Sub)),
        ))
        .parse_next(input)
    }

    (
        (opt(plus_or_minus), parse_multiplication),
        repeat(.., (plus_or_minus.map(|x| x.1), parse_multiplication)),
    )
        .map(|((add_sub, item), mut repeat): ((Option<_>, _), Vec<_>)| {
            if repeat.is_empty() && add_sub.map_or(true, |x| x.1 == AddOrSub::Add) {
                item
            } else {
                repeat.insert(0, (add_sub.map_or(AddOrSub::Add, |x| x.1), item));
                AstNode::add_sub(add_sub.map(|x| x.0), repeat)
            }
        })
        .parse_next(input)
}

fn parse_with_for<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    enum WithFor {
        With(AstNode),
        For(Vec<AstNode>),
    }

    fn parse_lesser_var_def<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
        separated_pair(parse_identifier, punct(Punctuation::Equals), parse_add_sub)
            .map(|(ident, expr)| {
                let span = ident.span().union(expr.span());
                AstNode::new(span, AstKind::VarDef { ident, expr })
            })
            .parse_next(input)
    }

    (
        parse_add_sub,
        repeat(
            ..,
            alt((
                (punct(Punctuation::With), parse_lesser_var_def).map(|(_, def)| WithFor::With(def)),
                (
                    punct(Punctuation::For),
                    separated(1.., parse_lesser_var_def, punct(Punctuation::Comma)),
                )
                    .map(|(_, defs): (_, Vec<_>)| WithFor::For(defs)),
            )),
        ),
    )
        .map(|(expr, trailing): (_, Vec<_>)| {
            trailing.into_iter().fold(expr, |expr, trail| match trail {
                WithFor::With(def) => {
                    let span = expr.span().union(def.span());
                    AstNode::new(span, AstKind::With { def, expr })
                }
                WithFor::For(defs) => {
                    let span = Span::union_n(defs.iter().map(|x| x.span())).union(expr.span());
                    AstNode::new(span, AstKind::For { expr, defs })
                }
            })
        })
        .parse_next(input)
}

pub fn parse_expr<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    parse_with_for(input)
}

pub fn parse_full_expr<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    terminated(parse_expr, eof).parse_next(input)
}

pub fn parse_cell<'a>(input: &mut Input<'a>) -> ParsedAstNode<'a> {
    alt((parse_var_def, parse_expr)).parse_next(input)
}

pub fn print_tree_err(source: &str, err: &TreeError<Input>, mut indent: usize) {
    let ind = "| ".repeat(indent);
    let ind = ind.white();
    indent += 1;
    match err {
        TreeError::Base(TreeErrorBase { kind, input, cause }) => {
            println!(
                "{ind}{}'{:?}' {}",
                "Base: ".bright_red(),
                kind,
                kind.description()
            );
            if input.is_empty() {
                println!("{ind}{}{}", "Input: ".green(), "EOF".blue());
            } else {
                println!(
                    "{ind}{}{}",
                    "Input: ".green(),
                    input[0].span.select(source).blue()
                );
            }
            if let Some(cause) = cause {
                println!("{ind}Cause: {cause}");
            }
        }
        TreeError::Stack { base, stack } => {
            if stack.is_empty() {
                print_tree_err(source, base, indent - 1);
            } else {
                println!("{ind}{}", "Stacked".bright_red());
                print_tree_err(source, base, indent);
                let ind = "| ".repeat(indent);
                let ind = ind.white();
                for frame in stack {
                    match frame {
                        TreeErrorFrame::Kind(TreeErrorBase {
                            kind,
                            input: _,
                            cause,
                        }) => {
                            println!(
                                "{ind}{}'{:?}' {}",
                                "Base: ".bright_red(),
                                kind,
                                kind.description()
                            );
                            // println!("{ind}{}{}", "Input: ".green(), input[0].span.from().blue());
                            if let Some(cause) = cause {
                                println!("{ind}Cause: {cause}");
                            }
                        }
                        TreeErrorFrame::Context(ctx) => {
                            // println!("{ind}{}", "Context ".bright_red());
                            // println!(
                            //     "{ind}{}{}",
                            //     "Input: ".green(),
                            //     ctx.input[0].span.from().blue()
                            // );
                            println!("{ind}{}{}", "Context: ".red(), ctx.context.yellow())
                        }
                    }
                }
            }
        }
        TreeError::Alt(trees) => {
            println!("{ind}{}", "Alt".bright_red());
            for tree in trees {
                print_tree_err(source, tree, indent);
            }
        }
    }
}

self_cell! {
    pub struct ParseOutcome {
        owner: Vec<Token>,

        #[covariant]
        dependent: ParseResult,
    }
}

pub type ParseResult<'a> = Result<AstNode, Option<ParseError<'a>>>;

pub fn parse_source(source: &str) -> Result<ParseOutcome, ContextError<LexError>> {
    let (lexed, err) = lexing::lex(source);
    if let Some(err) = err {
        return Err(err);
    }
    lexing::display_tokens(source, &lexed);
    Ok(ParseOutcome::new(lexed, |lexed| {
        let mut stream = lexed.as_slice();
        let parsed = parse_cell(&mut stream);
        match parsed {
            Ok(ast) => Ok(ast),
            Err(e) => match e {
                ErrMode::Backtrack(err) => Err(Some(err)),
                _ => {
                    println!("{}", "ERROR:".bright_red().bold());
                    println!("{e:#?}");
                    Err(None)
                }
            },
        }
    }))
}
