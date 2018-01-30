use std::str::FromStr;

mod grammar {
    include!(concat!(env!("OUT_DIR"), "/parse/grammar.rs"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

#[derive(Debug, Clone)]
pub enum ReplCommand {
    Eval(Box<Term>),
    Help,
    NoOp,
    Quit,
    TypeOf(Box<Term>),
}

impl FromStr for ReplCommand {
    type Err = ParseError;

    fn from_str(src: &str) -> Result<ReplCommand, ParseError> {
        grammar::parse_ReplCommand(src).map_err(|e| ParseError(format!("{}", e)))
    }
}

/// A module definition
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The name of the module
    pub name: String,
    /// The declarations contained in the module
    pub declarations: Vec<Declaration>,
}

impl FromStr for Module {
    type Err = ParseError;

    fn from_str(src: &str) -> Result<Module, ParseError> {
        grammar::parse_Module(src).map_err(|e| ParseError(format!("{}", e)))
    }
}

/// Top level declarations
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// Claims that a term abides by the given type
    Claim(String, Term),
    /// Declares the body of a term
    Definition(String, Vec<(String, Option<Box<Term>>)>, Term),
}

impl FromStr for Declaration {
    type Err = ParseError;

    fn from_str(src: &str) -> Result<Declaration, ParseError> {
        grammar::parse_Declaration(src).map_err(|e| ParseError(format!("{}", e)))
    }
}

/// The AST of the concrete syntax
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Var(String),
    Type,
    Ann(Box<Term>, Box<Term>),
    Lam(Vec<(String, Option<Box<Term>>)>, Box<Term>),
    Pi(String, Box<Term>, Box<Term>),
    Arrow(Box<Term>, Box<Term>),
    App(Box<Term>, Box<Term>),
}

impl FromStr for Term {
    type Err = ParseError;

    fn from_str(src: &str) -> Result<Term, ParseError> {
        grammar::parse_Term(src).map_err(|e| ParseError(format!("{}", e)))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use core::{Module, RcTerm, Term};
    use var::{Debruijn, Name, Named, Var};

    use super::{ParseError, Term as ParseTerm};

    fn parse(src: &str) -> RcTerm {
        RcTerm::from_parse(&src.parse().unwrap())
    }

    #[test]
    fn parse_prelude() {
        Module::from_parse(&include_str!("../../prelude.lp").parse().unwrap());
    }

    #[test]
    fn var() {
        assert_eq!(parse(r"x"), Term::from(Var::Free(Name::user("x"))).into());
    }

    #[test]
    fn var_kebab_case() {
        assert_eq!(
            parse(r"or-elim"),
            Term::from(Var::Free(Name::user("or-elim"))).into(),
        );
    }

    #[test]
    fn ty() {
        assert_eq!(parse(r"Type"), Term::Type.into());
    }

    #[test]
    fn ann() {
        assert_eq!(
            parse(r"Type : Type"),
            Term::Ann(Term::from(Term::Type).into(), Term::from(Term::Type).into(),).into(),
        );
    }

    #[test]
    fn ann_ann_left() {
        assert_eq!(
            parse(r"Type : Type : Type"),
            Term::Ann(
                Term::from(Term::Ann(
                    Term::from(Term::Type).into(),
                    Term::from(Term::Type).into(),
                )).into(),
                Term::from(Term::Type).into(),
            ).into(),
        );
    }

    #[test]
    fn ann_ann_right() {
        assert_eq!(
            parse(r"Type : (Type : Type)"),
            Term::Ann(
                Term::from(Term::Type).into(),
                Term::from(Term::Ann(
                    Term::from(Term::Type).into(),
                    Term::from(Term::Type).into(),
                )).into(),
            ).into(),
        );
    }

    #[test]
    fn ann_ann_ann() {
        assert_eq!(
            parse(r"(Type : Type) : (Type : Type)"),
            Term::Ann(
                Term::from(Term::Ann(
                    Term::from(Term::Type).into(),
                    Term::from(Term::Type).into(),
                )).into(),
                Term::from(Term::Ann(
                    Term::from(Term::Type).into(),
                    Term::from(Term::Type).into(),
                )).into(),
            ).into(),
        );
    }

    #[test]
    fn lam_ann() {
        let x = Name::user("x");

        assert_eq!(
            parse(r"\x : Type -> Type => x"),
            Term::Lam(
                Named(
                    x.clone(),
                    Some(
                        Term::from(Term::Pi(
                            Named(Name::Abstract, Term::from(Term::Type).into()),
                            Term::from(Term::Type).into(),
                        )).into()
                    ),
                ),
                Term::from(Var::Bound(Named(x, Debruijn(0)))).into(),
            ).into(),
        );
    }

    #[test]
    fn lam() {
        let x = Name::user("x");
        let y = Name::user("y");

        assert_eq!(
            parse(r"\x : (\y => y) => x"),
            Term::Lam(
                Named(
                    x.clone(),
                    Some(
                        Term::Lam(
                            Named(y.clone(), None),
                            Term::from(Var::Bound(Named(y, Debruijn(0)))).into(),
                        ).into()
                    ),
                ),
                Term::from(Var::Bound(Named(x, Debruijn(0)))).into(),
            ).into(),
        );
    }

    #[test]
    fn lam_multi() {
        assert_eq!(
            parse(r"\y (x : Type) z => x"),
            parse(r"\y => \x : Type => \z => x"),
        );
    }

    #[test]
    fn lam_lam_ann() {
        let x = Name::user("x");
        let y = Name::user("y");

        assert_eq!(
            parse(r"\x : Type => \y : Type => x"),
            Term::Lam(
                Named(x.clone(), Some(Term::from(Term::Type).into())),
                Term::Lam(
                    Named(y, Some(Term::from(Term::Type).into())),
                    Term::from(Var::Bound(Named(x, Debruijn(1)))).into(),
                ).into(),
            ).into(),
        );
    }

    #[test]
    fn arrow() {
        assert_eq!(
            parse(r"Type -> Type"),
            Term::Pi(
                Named(Name::Abstract, Term::from(Term::Type).into()),
                Term::from(Term::Type).into(),
            ).into(),
        );
    }

    #[test]
    fn pi() {
        let x = Name::user("x");

        assert_eq!(
            parse(r"(x : Type -> Type) -> x"),
            Term::Pi(
                Named(
                    x.clone(),
                    Term::from(Term::Pi(
                        Named(Name::Abstract, Term::from(Term::Type).into()),
                        Term::from(Term::Type).into(),
                    )).into(),
                ),
                Term::from(Var::Bound(Named(x, Debruijn(0)))).into(),
            ).into(),
        );
    }

    #[test]
    fn pi_pi() {
        let x = Name::user("x");
        let y = Name::user("y");

        assert_eq!(
            parse(r"(x : Type) -> (y : Type) -> x"),
            Term::Pi(
                Named(x.clone(), Term::from(Term::Type).into()),
                Term::from(Term::Pi(
                    Named(y, Term::from(Term::Type).into()),
                    Term::from(Var::Bound(Named(x, Debruijn(1)))).into(),
                )).into(),
            ).into(),
        );
    }

    #[test]
    fn pi_arrow() {
        let x = Name::user("x");

        assert_eq!(
            parse(r"(x : Type) -> x -> x"),
            Term::Pi(
                Named(x.clone(), Term::from(Term::Type).into()),
                Term::from(Term::Pi(
                    Named(
                        Name::Abstract,
                        Term::from(Var::Bound(Named(x.clone(), Debruijn(0)))).into(),
                    ),
                    Term::from(Var::Bound(Named(x, Debruijn(1)))).into(),
                )).into(),
            ).into(),
        );
    }

    #[test]
    fn pi_bad_ident() {
        let parse_result = ParseTerm::from_str("((x : Type) : Type) -> Type");

        assert_eq!(
            parse_result,
            Err(ParseError(String::from("identifier expected in pi type"))),
        );
    }

    #[test]
    fn lam_app() {
        let x = Name::user("x");
        let y = Name::user("y");

        assert_eq!(
            parse(r"\x : (Type -> Type) => \y : Type => x y"),
            Term::Lam(
                Named(
                    x.clone(),
                    Some(
                        Term::from(Term::Pi(
                            Named(Name::Abstract, Term::from(Term::Type).into()),
                            Term::from(Term::Type).into(),
                        )).into(),
                    ),
                ),
                Term::Lam(
                    Named(y.clone(), Some(Term::from(Term::Type).into())),
                    Term::App(
                        Term::from(Var::Bound(Named(x, Debruijn(1)))).into(),
                        Term::from(Var::Bound(Named(y, Debruijn(0)))).into(),
                    ).into(),
                ).into(),
            ).into(),
        );
    }

    #[test]
    fn id() {
        let x = Name::user("x");
        let a = Name::user("a");

        assert_eq!(
            parse(r"\a : Type => \x : a => x"),
            Term::Lam(
                Named(a.clone(), Some(Term::from(Term::Type).into())),
                Term::Lam(
                    Named(
                        x.clone(),
                        Some(Term::from(Var::Bound(Named(a, Debruijn(0)))).into()),
                    ),
                    Term::from(Var::Bound(Named(x, Debruijn(0)))).into(),
                ).into(),
            ).into(),
        );
    }

    #[test]
    fn id_ty() {
        let a = Name::user("a");

        assert_eq!(
            parse(r"(a : Type) -> a -> a"),
            Term::Pi(
                Named(a.clone(), Term::from(Term::Type).into()),
                Term::from(Term::Pi(
                    Named(
                        Name::Abstract,
                        Term::from(Var::Bound(Named(a.clone(), Debruijn(0)))).into(),
                    ),
                    Term::from(Var::Bound(Named(a, Debruijn(1)))).into(),
                )).into(),
            ).into(),
        );
    }

    #[test]
    fn id_ty_arr() {
        assert_eq!(
            parse(r"(a : Type) -> a -> a"),
            parse(r"(a : Type) -> (x : a) -> a"),
        )
    }
}
