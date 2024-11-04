use crate::lexing::{PairedPunct, Punctuation, Token, TokenKind};
use winnow::stream::ContainsToken;

impl ContainsToken<Token> for Punctuation {
    fn contains_token(&self, token: Token) -> bool {
        let TokenKind::Punct(punct) = &token.kind else {
            return false;
        };
        punct.eq(self)
    }
}

impl<const N: usize> ContainsToken<Token> for [Punctuation; N] {
    fn contains_token(&self, token: Token) -> bool {
        self.iter().any(|punct| punct.contains_token(token))
    }
}

impl ContainsToken<Token> for PairedPunct {
    fn contains_token(&self, token: Token) -> bool {
        let TokenKind::Paired(punct) = &token.kind else {
            return false;
        };
        punct.eq(self)
    }
}

impl ContainsToken<Token> for TokenKind {
    fn contains_token(&self, token: Token) -> bool {
        token.kind.eq(self)
    }
}

impl<const N: usize> ContainsToken<Token> for [TokenKind; N] {
    fn contains_token(&self, token: Token) -> bool {
        self.iter().any(|kind| token.kind.eq(kind))
    }
}

impl ContainsToken<Token> for &[TokenKind] {
    fn contains_token(&self, token: Token) -> bool {
        self.iter().any(|kind| token.kind.eq(kind))
    }
}
