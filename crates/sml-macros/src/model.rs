//! Validated, index-based view of a transition table.

use proc_macro2::Span;
use quote::{ToTokens, format_ident};
use syn::{Error, Expr, Ident, Result, Type, Visibility};

use crate::ast::{GuardExpr, MachineDef, Row, StateRef, Trigger};

pub struct Model {
    pub vis: Visibility,
    pub name: Ident,
    pub state_enum: Ident,
    pub context: Type,
    pub states: Vec<State>,
    pub initial: usize,
    pub history: bool,
    pub transitions: Vec<Transition>,
}

pub struct State {
    pub name: Ident,
    pub kind: StateKind,
    pub entry: Vec<Expr>,
    pub exit: Vec<Expr>,
}

pub enum StateKind {
    Simple,
    Composite(Box<Type>),
    Terminate,
}

pub struct Transition {
    pub source: usize,
    pub trigger: TransitionTrigger,
    pub guard: Option<GuardExpr>,
    pub actions: Vec<Expr>,
    pub target: Option<usize>,
}

pub enum TransitionTrigger {
    Anonymous,
    Event(Type),
    /// `None` matches any event type.
    Unexpected(Option<Type>),
}

impl State {
    pub fn composite(&self) -> Option<&Type> {
        match &self.kind {
            StateKind::Composite(ty) => Some(ty.as_ref()),
            StateKind::Simple | StateKind::Terminate => None,
        }
    }
}

impl Model {
    /// Field holding the sub-machine of composite state `state`.
    pub fn sub_field(state: &State) -> Ident {
        format_ident!("sub_{}", to_snake_case(&state.name.to_string()))
    }
}

#[derive(Default)]
struct Builder {
    states: Vec<State>,
    initial: Vec<(usize, Span)>,
    history: bool,
    transitions: Vec<Transition>,
    errors: Option<Error>,
}

pub fn build(def: MachineDef) -> Result<Model> {
    let mut builder = Builder::default();
    for row in def.rows {
        if let Err(err) = builder.add_row(row) {
            builder.push_error(err);
        }
    }
    let initial = builder.resolve_initial(&def.name);
    if let Some(errors) = builder.errors {
        return Err(errors);
    }
    let context = def.context.unwrap_or_else(|| syn::parse_quote!(()));
    Ok(Model {
        state_enum: format_ident!("{}State", def.name),
        vis: def.vis,
        name: def.name,
        context,
        states: builder.states,
        initial,
        history: builder.history,
        transitions: builder.transitions,
    })
}

impl Builder {
    fn push_error(&mut self, err: Error) {
        match &mut self.errors {
            Some(errors) => errors.combine(err),
            None => self.errors = Some(err),
        }
    }

    fn add_row(&mut self, row: Row) -> Result<()> {
        if let StateRef::Terminate(name) = &row.source {
            return Err(Error::new(
                name.span(),
                "`X` is the terminate state; it has no transitions",
            ));
        }
        let source = self.intern(&row.source)?;
        if let Some(span) = row.initial {
            self.initial.push((source, span));
        }
        if let Some(span) = row.history {
            if row.initial.is_none() {
                return Err(Error::new(
                    span,
                    "`(H)` goes on the initial state: `*Idle(H)`",
                ));
            }
            self.history = true;
        }
        let target = row
            .target
            .as_ref()
            .map(|target| self.intern(target))
            .transpose()?;
        let trigger = match row.trigger {
            Trigger::OnEntry(span) => return self.add_state_action(source, row, span, true),
            Trigger::OnExit(span) => return self.add_state_action(source, row, span, false),
            Trigger::Anonymous if target.is_none() => {
                return Err(Error::new(
                    row.span,
                    "anonymous transition needs a target: `= State`",
                ));
            }
            Trigger::Anonymous => TransitionTrigger::Anonymous,
            Trigger::Event(ty) => TransitionTrigger::Event(ty),
            Trigger::Unexpected(ty) => TransitionTrigger::Unexpected(ty),
        };
        self.transitions.push(Transition {
            source,
            trigger,
            guard: row.guard,
            actions: row.actions,
            target,
        });
        Ok(())
    }

    fn add_state_action(&mut self, state: usize, row: Row, span: Span, entry: bool) -> Result<()> {
        if row.guard.is_some() || row.target.is_some() {
            return Err(Error::new(
                span,
                "`on_entry`/`on_exit` rows take only an action",
            ));
        }
        let state = &mut self.states[state];
        if entry {
            state.entry.extend(row.actions);
        } else {
            state.exit.extend(row.actions);
        }
        Ok(())
    }

    fn intern(&mut self, state: &StateRef) -> Result<usize> {
        let name = state.name();
        let Some(index) = self.states.iter().position(|known| known.name == *name) else {
            let kind = match state {
                StateRef::Simple(_) => StateKind::Simple,
                StateRef::Composite { ty, .. } => StateKind::Composite(ty.clone()),
                StateRef::Terminate(_) => StateKind::Terminate,
            };
            let entry = Vec::new();
            let exit = Vec::new();
            self.states.push(State {
                name: name.clone(),
                kind,
                entry,
                exit,
            });
            return Ok(self.states.len() - 1);
        };
        let StateRef::Composite { ty, .. } = state else {
            return Ok(index);
        };
        let known = &mut self.states[index].kind;
        match known {
            StateKind::Simple => *known = StateKind::Composite(ty.clone()),
            StateKind::Composite(old) => {
                if old.to_token_stream().to_string() != ty.to_token_stream().to_string() {
                    return Err(Error::new(
                        name.span(),
                        "two sub-machine types share this state name",
                    ));
                }
            }
            StateKind::Terminate => return Err(Error::new(name.span(), "`X` is reserved")),
        }
        Ok(index)
    }

    fn resolve_initial(&mut self, machine: &Ident) -> usize {
        let Some(&(first, _)) = self.initial.first() else {
            self.push_error(Error::new(
                machine.span(),
                "no initial state; mark one with `*`",
            ));
            return 0;
        };
        let extra: Vec<Span> = self
            .initial
            .iter()
            .filter(|(state, _)| *state != first)
            .map(|(_, span)| *span)
            .collect();
        for span in extra {
            self.push_error(Error::new(
                span,
                "second initial state: orthogonal regions are not supported",
            ));
        }
        first
    }
}

pub fn to_snake_case(name: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.extend(ch.to_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

#[cfg(test)]
mod tests {
    use super::to_snake_case;

    #[test]
    fn snake_case_splits_on_uppercase_without_leading_underscore() {
        assert_eq!(to_snake_case("Idle"), "idle");
        assert_eq!(to_snake_case("WaitingForAck"), "waiting_for_ack");
        assert_eq!(to_snake_case("lower"), "lower");
        assert_eq!(to_snake_case(""), "");
    }
}
