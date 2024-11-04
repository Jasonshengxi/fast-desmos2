use crate::utils::OptExt;
use std::fmt::{Debug, Formatter};
use std::ptr;

#[derive(Clone, Copy)]
pub struct StrSpan<'a> {
    source: &'a str,
    span: Span,
}

impl Debug for StrSpan<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StrSpan").field(&self.as_str()).finish()
    }
}

impl<'a> StrSpan<'a> {
    pub fn as_str(&self) -> &'a str {
        self.span.select(self.source)
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn span(&self) -> Span {
        self.span
    }
    pub fn from(&self) -> usize {
        self.span.from
    }

    pub fn to(&self) -> usize {
        self.span.to
    }

    pub fn new(source: &'a str, span: Span) -> Self {
        Self { source, span }
    }

    pub fn union(self, other: Self) -> Self {
        assert!(ptr::eq(self.source, other.source));
        Self {
            source: self.source,
            span: self.span.union(other.span),
        }
    }

    pub fn union_n(spans: impl IntoIterator<Item = Self>) -> Self {
        spans.into_iter().reduce(Self::union).unwrap_unreach()
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Span {
    pub from: usize,
    pub to: usize,
}

impl ariadne::Span for Span {
    type SourceId = ();

    fn source(&self) -> &Self::SourceId {
        &()
    }

    fn start(&self) -> usize {
        self.from
    }

    fn end(&self) -> usize {
        self.to
    }
}

impl Span {
    pub const fn new(from: usize, to: usize) -> Self {
        assert!(to >= from);
        Self { from, to }
    }

    pub const fn len(&self) -> usize {
        self.to - self.from
    }

    pub const fn from_len(from: usize, len: usize) -> Self {
        Self::new(from, from + len)
    }

    pub fn select<'a>(&self, source: &'a str) -> &'a str {
        &source[self.from..self.to]
    }

    pub const EMPTY: Self = Self::new(0, 0);

    pub const fn is_empty(&self) -> bool {
        self.to <= self.from
    }

    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            other
        } else if other.is_empty() {
            self
        } else {
            Self {
                from: self.from.min(other.from),
                to: self.to.max(other.to),
            }
        }
    }

    pub fn union_n(parts: impl IntoIterator<Item = Self>) -> Self {
        parts
            .into_iter()
            .reduce(|acc, span| acc.union(span))
            .unwrap_or_default()
    }
}
