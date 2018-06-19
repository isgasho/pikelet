//! The syntax of the language

use nameless::{BoundPattern, BoundTerm, BoundVar, FreeVar, ScopeState};
use std::fmt;

pub mod concrete;
pub mod context;
pub mod core;
pub mod parse;
pub mod pretty;
pub mod prim;
pub mod raw;
pub mod translation;

/// A universe level
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, BoundTerm)]
pub struct Level(pub u32);

impl Level {
    pub fn succ(self) -> Level {
        Level(self.0 + 1)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A record label
///
/// Labels are significant when comparing for alpha-equality, both in terms and
/// in patterns
#[derive(Debug, Clone, PartialEq)]
pub struct Label(pub FreeVar);

impl BoundTerm for Label {
    fn term_eq(&self, other: &Label) -> bool {
        match (self.0.ident(), other.0.ident()) {
            (Some(lhs), Some(rhs)) => lhs == rhs,
            (_, _) => FreeVar::term_eq(&self.0, &other.0),
        }
    }
}

impl BoundPattern for Label {
    fn pattern_eq(&self, other: &Label) -> bool {
        Label::term_eq(self, other)
    }

    fn freshen(&mut self) -> Vec<FreeVar> {
        self.0.freshen()
    }

    fn rename(&mut self, perm: &[FreeVar]) {
        self.0.rename(perm)
    }

    fn on_free(&self, state: ScopeState, name: &FreeVar) -> Option<BoundVar> {
        self.0.on_free(state, name)
    }

    fn on_bound(&self, state: ScopeState, name: BoundVar) -> Option<FreeVar> {
        self.0.on_bound(state, name)
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}