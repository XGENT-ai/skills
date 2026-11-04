//! Model expressions: single-key JSON operator objects compiled to a typed tree.
use super::Problems;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Val {
    Bool(bool),
    Int(i64),
    Str(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ty {
    Bool,
    Int,
    Str,
}

impl Ty {
    fn name(self) -> &'static str {
        match self {
            Ty::Bool => "boolean",
            Ty::Int => "integer",
            Ty::Str => "string",
        }
    }
}

impl Val {
    pub fn ty(self) -> Ty {
        match self {
            Val::Bool(_) => Ty::Bool,
            Val::Int(_) => Ty::Int,
            Val::Str(_) => Ty::Str,
        }
    }
}

/// Every string value of one model, interned so states stay small and hashable.
#[derive(Default)]
pub struct Strings {
    names: Vec<String>,
    index: HashMap<String, u32>,
}

impl Strings {
    pub fn intern(&mut self, value: &str) -> u32 {
        if let Some(id) = self.index.get(value) {
            return *id;
        }
        let id = self.names.len() as u32;
        self.names.push(value.into());
        self.index.insert(value.into(), id);
        id
    }
    pub fn get(&self, id: u32) -> &str {
        &self.names[id as usize]
    }
    /// The text a label template shows: strings as written, numbers in decimal.
    pub fn display(&self, value: Val) -> String {
        match value {
            Val::Bool(value) => value.to_string(),
            Val::Int(value) => value.to_string(),
            Val::Str(id) => self.get(id).into(),
        }
    }
    /// The form a default state name and evidence use: strings as JSON literals.
    pub fn literal(&self, value: Val) -> String {
        match value {
            Val::Str(id) => Value::String(self.get(id).into()).to_string(),
            other => self.display(other),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Domain {
    Values(Vec<Val>),
    Range(i64, i64),
}

impl Domain {
    pub fn ty(&self) -> Ty {
        match self {
            Domain::Values(values) => values[0].ty(),
            Domain::Range(..) => Ty::Int,
        }
    }
    pub fn contains(&self, value: Val) -> bool {
        match (self, value) {
            (Domain::Values(values), value) => values.contains(&value),
            (Domain::Range(lo, hi), Val::Int(value)) => (*lo..=*hi).contains(&value),
            _ => false,
        }
    }
}

/// A named variable or action parameter with its finite domain.
#[derive(Clone, Debug)]
pub struct Slot {
    pub name: String,
    pub domain: Domain,
}

#[derive(Clone, Copy, Debug)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Lit(Val),
    Var(usize),
    Next(usize),
    Param(usize),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Implies(Box<Expr>, Box<Expr>),
    Cmp(Cmp, Box<Expr>, Box<Expr>),
    In(Box<Expr>, Vec<Val>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
}

/// Integer arithmetic left the signed 64-bit range.
#[derive(Debug)]
pub struct Overflow;

pub struct Env<'a> {
    pub state: &'a [Val],
    pub next: &'a [Val],
    pub params: &'a [Val],
}

impl Expr {
    pub fn eval(&self, env: &Env<'_>) -> Result<Val, Overflow> {
        let int = |expr: &Expr| match expr.eval(env)? {
            Val::Int(value) => Ok(value),
            _ => unreachable!("type-checked integer"),
        };
        let truth = |expr: &Expr| Ok(expr.eval(env)? == Val::Bool(true));
        Ok(match self {
            Expr::Lit(value) => *value,
            Expr::Var(index) => env.state[*index],
            Expr::Next(index) => env.next[*index],
            Expr::Param(index) => env.params[*index],
            Expr::Not(expr) => Val::Bool(!truth(expr)?),
            Expr::And(items) => {
                for item in items {
                    if !truth(item)? {
                        return Ok(Val::Bool(false));
                    }
                }
                Val::Bool(true)
            }
            Expr::Or(items) => {
                for item in items {
                    if truth(item)? {
                        return Ok(Val::Bool(true));
                    }
                }
                Val::Bool(false)
            }
            Expr::Implies(a, b) => Val::Bool(!truth(a)? || truth(b)?),
            Expr::Cmp(op, a, b) => {
                let (a, b) = (a.eval(env)?, b.eval(env)?);
                Val::Bool(match op {
                    Cmp::Eq => a == b,
                    Cmp::Ne => a != b,
                    Cmp::Lt => a < b,
                    Cmp::Le => a <= b,
                    Cmp::Gt => a > b,
                    Cmp::Ge => a >= b,
                })
            }
            Expr::In(expr, list) => Val::Bool(list.contains(&expr.eval(env)?)),
            Expr::Add(a, b) => Val::Int(int(a)?.checked_add(int(b)?).ok_or(Overflow)?),
            Expr::Sub(a, b) => Val::Int(int(a)?.checked_sub(int(b)?).ok_or(Overflow)?),
            Expr::If(c, a, b) => {
                if truth(c)? {
                    a.eval(env)?
                } else {
                    b.eval(env)?
                }
            }
        })
    }
}

/// Where an expression appears decides which names it may use.
pub struct Scope<'a> {
    pub variables: &'a [Slot],
    /// The parameters of the enclosing action; `None` outside any action.
    pub params: Option<&'a [Slot]>,
    pub next: bool,
}

pub fn literal(value: &Value, strings: &mut Strings) -> Option<Val> {
    match value {
        Value::Bool(value) => Some(Val::Bool(*value)),
        Value::String(value) => Some(Val::Str(strings.intern(value))),
        Value::Number(number) => number
            .as_i64()
            .or_else(|| {
                number
                    .as_f64()
                    .filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
                    .map(|f| f as i64)
            })
            .map(Val::Int),
        _ => None,
    }
}

pub(crate) struct Compiler<'a, 'b> {
    pub scope: Scope<'a>,
    pub strings: &'b mut Strings,
    pub problems: &'b mut Problems,
}

impl Compiler<'_, '_> {
    /// Compile an expression that must produce `want`.
    pub fn expect(&mut self, value: &Value, path: &str, want: Ty) -> Option<Expr> {
        let (expr, ty) = self.compile(value, path)?;
        self.check(ty, want, path).then_some(expr)
    }

    fn check(&mut self, actual: Ty, want: Ty, path: &str) -> bool {
        if actual != want {
            self.problems.push(
                "statechart/expression-type",
                path,
                format!(
                    "Expected a {} expression, found {}",
                    want.name(),
                    actual.name()
                ),
                json!({"expected":want.name(),"actual":actual.name()}),
            );
        }
        actual == want
    }

    fn slot(&self, expr: &Expr) -> Option<&Slot> {
        match expr {
            Expr::Var(index) | Expr::Next(index) => Some(&self.scope.variables[*index]),
            Expr::Param(index) => self.scope.params.map(|params| &params[*index]),
            _ => None,
        }
    }

    /// A literal compared with, or listed for, a variable or parameter must be
    /// one of its values; a misspelled value would otherwise silently never match.
    fn in_domain(&mut self, slot: &Expr, value: Val, path: &str) {
        let Some(slot) = self.slot(slot) else { return };
        if slot.domain.ty() == value.ty() && !slot.domain.contains(value) {
            let (name, shown) = (slot.name.clone(), self.strings.literal(value));
            self.problems.push(
                "statechart/value-not-in-domain",
                path,
                format!("{shown} is not a value of {name}"),
                json!({"name":name,"value":shown}),
            );
        }
    }

    pub fn compile(&mut self, value: &Value, path: &str) -> Option<(Expr, Ty)> {
        if let Some(value) = literal(value, self.strings) {
            return Some((Expr::Lit(value), value.ty()));
        }
        let (key, arg) = value.as_object()?.iter().next()?;
        let at = format!("{path}/{key}");
        let item = |index: usize| format!("{at}/{index}");
        match key.as_str() {
            "var" | "next" => {
                let name = arg.as_str()?;
                if key == "next" && !self.scope.next {
                    self.problems.push(
                        "statechart/next-outside-step",
                        &at,
                        "next reads the value after a step and is only available in view.steps[].when",
                        Value::Null,
                    );
                    return None;
                }
                let Some(index) = self
                    .scope
                    .variables
                    .iter()
                    .position(|slot| slot.name == name)
                else {
                    self.problems.push(
                        "statechart/unknown-variable",
                        &at,
                        format!("Unknown variable {name}"),
                        json!({"name":name}),
                    );
                    return None;
                };
                let ty = self.scope.variables[index].domain.ty();
                Some((
                    if key == "var" {
                        Expr::Var(index)
                    } else {
                        Expr::Next(index)
                    },
                    ty,
                ))
            }
            "param" => {
                let name = arg.as_str()?;
                let found = self
                    .scope
                    .params
                    .and_then(|params| params.iter().position(|slot| slot.name == name));
                let Some(index) = found else {
                    self.problems.push(
                        "statechart/unknown-param",
                        &at,
                        format!("{name} is not a pick parameter of this action"),
                        json!({"name":name}),
                    );
                    return None;
                };
                Some((Expr::Param(index), self.scope.params?[index].domain.ty()))
            }
            "not" => Some((
                Expr::Not(Box::new(self.expect(arg, &at, Ty::Bool)?)),
                Ty::Bool,
            )),
            "and" | "or" => {
                let items: Vec<_> = arg
                    .as_array()?
                    .iter()
                    .enumerate()
                    .map(|(index, value)| self.expect(value, &item(index), Ty::Bool))
                    .collect();
                let items = items.into_iter().collect::<Option<Vec<_>>>()?;
                Some((
                    if key == "and" {
                        Expr::And(items)
                    } else {
                        Expr::Or(items)
                    },
                    Ty::Bool,
                ))
            }
            "implies" => {
                let (a, b) = (
                    self.expect(&arg[0], &item(0), Ty::Bool),
                    self.expect(&arg[1], &item(1), Ty::Bool),
                );
                Some((Expr::Implies(Box::new(a?), Box::new(b?)), Ty::Bool))
            }
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
                let (a, b) = (
                    self.compile(&arg[0], &item(0)),
                    self.compile(&arg[1], &item(1)),
                );
                let ((a, ta), (b, tb)) = (a?, b?);
                let ordered = !matches!(key.as_str(), "eq" | "ne");
                if ordered {
                    let (ok_a, ok_b) = (
                        self.check(ta, Ty::Int, &item(0)),
                        self.check(tb, Ty::Int, &item(1)),
                    );
                    if !(ok_a && ok_b) {
                        return None;
                    }
                } else {
                    if !self.check(tb, ta, &item(1)) {
                        return None;
                    }
                    for (slot, other, at) in [(&a, &b, item(1)), (&b, &a, item(0))] {
                        if let Expr::Lit(value) = other {
                            self.in_domain(slot, *value, &at);
                        }
                    }
                }
                let op = match key.as_str() {
                    "eq" => Cmp::Eq,
                    "ne" => Cmp::Ne,
                    "lt" => Cmp::Lt,
                    "le" => Cmp::Le,
                    "gt" => Cmp::Gt,
                    _ => Cmp::Ge,
                };
                Some((Expr::Cmp(op, Box::new(a), Box::new(b)), Ty::Bool))
            }
            "in" => {
                let (expr, ty) = self.compile(&arg[0], &item(0))?;
                let mut list = Vec::new();
                for (index, value) in arg[1].as_array()?.iter().enumerate() {
                    let at = format!("{at}/1/{index}");
                    let value = literal(value, self.strings)?;
                    if !self.check(value.ty(), ty, &at) {
                        return None;
                    }
                    self.in_domain(&expr, value, &at);
                    list.push(value);
                }
                Some((Expr::In(Box::new(expr), list), Ty::Bool))
            }
            "add" | "sub" => {
                let (a, b) = (
                    self.expect(&arg[0], &item(0), Ty::Int),
                    self.expect(&arg[1], &item(1), Ty::Int),
                );
                let (a, b) = (Box::new(a?), Box::new(b?));
                Some((
                    if key == "add" {
                        Expr::Add(a, b)
                    } else {
                        Expr::Sub(a, b)
                    },
                    Ty::Int,
                ))
            }
            "if" => {
                let condition = self.expect(&arg[0], &item(0), Ty::Bool);
                let (a, b) = (
                    self.compile(&arg[1], &item(1)),
                    self.compile(&arg[2], &item(2)),
                );
                let ((a, ta), (b, tb)) = (a?, b?);
                (self.check(tb, ta, &item(2))).then_some(())?;
                Some((Expr::If(Box::new(condition?), Box::new(a), Box::new(b)), ta))
            }
            _ => None,
        }
    }
}
