use crate::parser::term_pretty_print;
use std::{fmt::Debug, rc::Rc};

pub mod infer;

#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Abstraction {
    pub var_ty: TermRef,
    pub body: TermRef,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    Axiom { ty: TermRef, unique_name: String },
    Universe { index: usize },
    Forall(Abstraction),
    Fun(Abstraction),
    Var { index: usize },
    Number { value: i32 },
    App { func: TermRef, op: TermRef },
    Wild { index: usize },
}

pub type TermRef = Rc<Term>;

impl Debug for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut name_stack = vec![];
        f.write_str(&term_pretty_print(
            Rc::new(self.clone()),
            &mut name_stack,
            (200, 200),
        ))
    }
}

#[macro_export]
macro_rules! term_ref {
    {$input:expr} => (($input).clone());
    {$($i:tt)*} => (crate::TermRef::new(crate::term!($( $i)*)));
}

#[macro_export]
macro_rules! term {
    {forall $ty:expr , $($i:tt)*} => (crate::Term::Forall(crate::Abstraction { var_ty: term_ref!($ty), body: (term_ref!($( $i)*)) }));
    {fun $ty:expr , $($i:tt)*} => (crate::Term::Fun(crate::Abstraction { var_ty: term_ref!($ty), body: (term_ref!($( $i)*)) }));
    {axiom $name:expr , $($i:tt)*} => (crate::Term::Axiom { ty: term_ref!($( $i)*), unique_name: ($name).to_string() });
    {universe $input:expr} => (crate::Term::Universe { index: ($input) });
    {v $input:expr} => (crate::Term::Var { index: ($input) });
    {n $input:expr} => (crate::Term::Number { value: ($input) });
    {_ $input:expr} => (crate::Term::Wild { index: ($input) });
    {$input:expr} => ($input);
}

#[macro_export]
macro_rules! app_ref {
    {$($i:tt)*} => (crate::TermRef::new(crate::app!($( $i)*)));
}

#[macro_export]
macro_rules! app {
    ( $x:expr , $y:expr ) => {
        crate::Term::App {
            func: ($x).clone(),
            op: ($y).clone(),
        }
    };
    ( $x:expr , $y:expr, $z:expr ) => {
        crate::Term::App {
            func: crate::TermRef::new(crate::Term::App {
                func: ($x).clone(),
                op: ($y).clone(),
            }),
            op: ($z).clone(),
        }
    };
    ( $x:expr , $y:expr, $z:expr, $w:expr ) => {
        crate::Term::App {
            func: crate::TermRef::new(crate::Term::App {
                func: TermRef::new(crate::Term::App {
                    func: ($x).clone(),
                    op: ($y).clone(),
                }),
                op: ($z).clone(),
            }),
            op: ($w).clone(),
        }
    };
}

#[derive(Debug)]
pub enum Error {
    BadTerm,
    TypeMismatch(TermRef, TermRef),
    IsNotFunc,
    ContainsWild,
    IsNotUniverse,
}

pub type Result<T> = std::result::Result<T, Error>;

use Error::*;

pub fn contains_wild(t: &TermRef) -> bool {
    match t.as_ref() {
        Term::Axiom { .. } | Term::Universe { .. } | Term::Var { .. } | Term::Number { .. } => {
            false
        }
        Term::App { func, op } => contains_wild(func) || contains_wild(op),
        Term::Forall(Abstraction { var_ty, body }) | Term::Fun(Abstraction { var_ty, body }) => {
            contains_wild(var_ty) || contains_wild(body)
        }
        Term::Wild { .. } => true,
    }
}

pub fn remove_unused_var(t: &TermRef, depth: usize) -> Option<TermRef> {
    fn for_abs(Abstraction { var_ty, body }: &Abstraction, depth: usize) -> Option<Abstraction> {
        Some(Abstraction {
            var_ty: remove_unused_var(&var_ty, depth)?,
            body: remove_unused_var(&body, depth + 1)?,
        })
    }
    Some(match t.as_ref() {
        Term::Axiom { .. } | Term::Universe { .. } | Term::Wild { .. } | Term::Number { .. } => {
            t.clone()
        }
        Term::App { func, op } => {
            let func = remove_unused_var(func, depth)?;
            let op = remove_unused_var(op, depth)?;
            app_ref!(func, op)
        }
        Term::Forall(x) => TermRef::new(Term::Forall(for_abs(x, depth)?)),
        Term::Fun(x) => TermRef::new(Term::Fun(for_abs(x, depth)?)),
        Term::Var { index } => {
            let i = *index;
            if i == depth {
                return None;
            } else if i < depth {
                term_ref!(v i)
            } else {
                term_ref!(v i - 1)
            }
        }
    })
}

fn get_universe(t: TermRef) -> Result<usize> {
    if let Term::Universe { index } = t.as_ref() {
        Ok(*index)
    } else {
        Err(IsNotUniverse)
    }
}

fn deny_wild(t: &TermRef) -> Result<()> {
    if contains_wild(t) {
        Err(ContainsWild)
    } else {
        Ok(())
    }
}

fn fill_wild(t: TermRef, f: &impl Fn(usize) -> TermRef) -> TermRef {
    match t.as_ref() {
        Term::Axiom { .. } | Term::Universe { .. } | Term::Var { .. } | Term::Number { .. } => t,
        Term::App { func, op } => app_ref!(fill_wild(func.clone(), f), fill_wild(op.clone(), f)),
        Term::Forall(Abstraction { var_ty, body }) => {
            term_ref!(forall fill_wild(var_ty.clone(), f), fill_wild(body.clone(), f))
        }
        Term::Fun(Abstraction { var_ty, body }) => {
            term_ref!(fun fill_wild(var_ty.clone(), f), fill_wild(body.clone(), f))
        }
        Term::Wild { index } => f(*index),
    }
}

fn normalize(t: TermRef) -> TermRef {
    fn for_abs(a: Abstraction) -> Abstraction {
        Abstraction {
            var_ty: normalize(a.var_ty),
            body: normalize(a.body),
        }
    }
    match t.as_ref() {
        Term::Var { .. }
        | Term::Axiom { .. }
        | Term::Universe { .. }
        | Term::Number { .. }
        | Term::Wild { .. } => t,
        Term::Forall(x) => TermRef::new(Term::Forall(for_abs(x.clone()))),
        Term::Fun(x) => TermRef::new(Term::Fun(for_abs(x.clone()))),
        Term::App { func, op } => {
            let func = normalize(func.clone());
            if let Term::Fun(x) = func.as_ref() {
                return normalize(subst(x.body.clone(), op.clone()));
            }
            let op = normalize(op.clone());
            app_ref!(func, op)
        }
    }
}

pub fn subst(exp: TermRef, to_put: TermRef) -> TermRef {
    fn inner(exp: TermRef, to_put: TermRef, i: usize) -> TermRef {
        match exp.as_ref() {
            Term::Var { index } if *index == i => to_put,
            Term::Var { .. }
            | Term::Axiom { .. }
            | Term::Universe { .. }
            | Term::Number { .. }
            | Term::Wild { .. } => exp,
            Term::Forall(Abstraction { var_ty, body }) => term_ref!(forall
                inner(var_ty.clone(), to_put.clone(), i),
                inner(body.clone(), to_put, i + 1)
            ),
            Term::Fun(Abstraction { var_ty, body }) => term_ref!(fun
                inner(var_ty.clone(), to_put.clone(), i),
                inner(body.clone(), to_put, i + 1)
            ),
            Term::App { func, op } => TermRef::new(Term::App {
                func: inner(func.clone(), to_put.clone(), i),
                op: inner(op.clone(), to_put, i),
            }),
        }
    }
    inner(exp, to_put, 0)
}

pub fn increase_foreign_vars(term: TermRef, depth: usize) -> TermRef {
    match term.as_ref() {
        Term::Var { index } if *index >= depth => TermRef::new(Term::Var { index: index + 1 }),
        Term::Axiom { .. }
        | Term::Universe { .. }
        | Term::Number { .. }
        | Term::Var { .. }
        | Term::Wild { .. } => term,
        Term::Forall(Abstraction { var_ty, body }) => {
            let var_ty = increase_foreign_vars(var_ty.clone(), depth);
            let body = increase_foreign_vars(body.clone(), depth + 1);
            term_ref!(forall var_ty, body)
        }
        Term::Fun(Abstraction { var_ty, body }) => {
            let var_ty = increase_foreign_vars(var_ty.clone(), depth);
            let body = increase_foreign_vars(body.clone(), depth + 1);
            term_ref!(fun var_ty, body)
        }
        Term::App { func, op } => {
            let func = increase_foreign_vars(func.clone(), depth);
            let op = increase_foreign_vars(op.clone(), depth);
            TermRef::new(Term::App { func, op })
        }
    }
}

pub fn type_of(term: TermRef) -> Result<TermRef> {
    deny_wild(&term)?;
    infer::type_of_inner(term, &[], &mut infer::InferResults::new(0))
}
