//! Syntax of the `sml!` transition table.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{BinOp, Error, Expr, Ident, Result, Token, Type, UnOp, Visibility};

mod kw {
    syn::custom_keyword!(machine);
    syn::custom_keyword!(state);
    syn::custom_keyword!(on_entry);
    syn::custom_keyword!(on_exit);
    syn::custom_keyword!(unexpected_event);
    syn::custom_keyword!(H);
}

/// `[pub] machine Name[<Context>]; rows...`
pub struct MachineDef {
    pub vis: Visibility,
    pub name: Ident,
    pub context: Option<Type>,
    pub rows: Vec<Row>,
}

/// One table row: `[*]src[(H)] [+ trigger] [[guard]] [/ actions] [= dst]`.
pub struct Row {
    pub initial: Option<Span>,
    pub source: StateRef,
    pub history: Option<Span>,
    pub trigger: Trigger,
    pub guard: Option<GuardExpr>,
    pub actions: Vec<Expr>,
    pub target: Option<StateRef>,
    pub span: Span,
}

/// State reference.
pub enum StateRef {
    Simple(Ident),
    Composite { name: Ident, ty: Box<Type> },
    Terminate(Ident),
}

/// What makes a row fire.
pub enum Trigger {
    Anonymous,
    Event(Type),
    OnEntry(Span),
    OnExit(Span),
    /// `None` matches any event type.
    Unexpected(Option<Type>),
}

/// Boolean combination of guard callables.
pub enum GuardExpr {
    Leaf(Expr),
    Not(Box<GuardExpr>),
    And(Box<GuardExpr>, Box<GuardExpr>),
    Or(Box<GuardExpr>, Box<GuardExpr>),
}

impl StateRef {
    pub fn name(&self) -> &Ident {
        match self {
            Self::Simple(name) | Self::Composite { name, .. } | Self::Terminate(name) => name,
        }
    }
}

impl Parse for MachineDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis = input.parse()?;
        input.parse::<kw::machine>()?;
        let name = input.parse()?;
        let mut context = None;
        if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            context = Some(input.parse()?);
            input.parse::<Token![>]>()?;
        }
        input.parse::<Token![;]>()?;
        let rows = Punctuated::<Row, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            vis,
            name,
            context,
            rows: rows.into_iter().collect(),
        })
    }
}

impl Parse for Row {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();
        let initial = input.parse::<Option<Token![*]>>()?.map(|star| star.span);
        let source = input.parse()?;
        let history = parse_history(input)?;
        let trigger = if input.parse::<Option<Token![+]>>()?.is_some() {
            input.parse()?
        } else {
            Trigger::Anonymous
        };
        let guard = parse_guard(input)?;
        let actions = parse_actions(input)?;
        let target = if input.parse::<Option<Token![=]>>()?.is_some() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            initial,
            source,
            history,
            trigger,
            guard,
            actions,
            target,
            span,
        })
    }
}

impl Parse for StateRef {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::state) {
            input.parse::<kw::state>()?;
            input.parse::<Token![<]>()?;
            let ty: Type = input.parse()?;
            input.parse::<Token![>]>()?;
            let name = composite_name(&ty)?;
            return Ok(Self::Composite {
                name,
                ty: Box::new(ty),
            });
        }
        let name: Ident = input.parse()?;
        if name == "X" {
            return Ok(Self::Terminate(name));
        }
        Ok(Self::Simple(name))
    }
}

impl Parse for Trigger {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(kw::on_entry) {
            let span = input.parse::<kw::on_entry>()?.span;
            parse_wildcard_arg(input)?;
            return Ok(Self::OnEntry(span));
        }
        if input.peek(kw::on_exit) {
            let span = input.parse::<kw::on_exit>()?.span;
            parse_wildcard_arg(input)?;
            return Ok(Self::OnExit(span));
        }
        if input.peek(kw::unexpected_event) {
            input.parse::<kw::unexpected_event>()?;
            return parse_unexpected_arg(input);
        }
        let event = input.parse().map_err(|err| {
            Error::new(
                err.span(),
                "expected event type, `on_entry`, `on_exit` or `unexpected_event`",
            )
        })?;
        Ok(Self::Event(event))
    }
}

/// Variant name of `state<path::Name>`: the last path segment.
fn composite_name(ty: &Type) -> Result<Ident> {
    if let Type::Path(path) = ty
        && path.qself.is_none()
        && let Some(last) = path.path.segments.last()
        && last.ident != "X"
    {
        return Ok(last.ident.clone());
    }
    Err(Error::new(
        ty.span(),
        "expected a sub-machine type path, e.g. `state<Inner>`",
    ))
}

fn parse_history(input: ParseStream) -> Result<Option<Span>> {
    if !input.peek(syn::token::Paren) {
        return Ok(None);
    }
    let content;
    let paren = syn::parenthesized!(content in input);
    content.parse::<kw::H>()?;
    Ok(Some(paren.span.join()))
}

/// Optional `<_>` after `on_entry` / `on_exit`, accepted for SML familiarity.
fn parse_wildcard_arg(input: ParseStream) -> Result<()> {
    if input.parse::<Option<Token![<]>>()?.is_some() {
        input.parse::<Token![_]>()?;
        input.parse::<Token![>]>()?;
    }
    Ok(())
}

fn parse_unexpected_arg(input: ParseStream) -> Result<Trigger> {
    if input.parse::<Option<Token![<]>>()?.is_none() {
        return Ok(Trigger::Unexpected(None));
    }
    let ty = if input.parse::<Option<Token![_]>>()?.is_some() {
        None
    } else {
        Some(input.parse()?)
    };
    input.parse::<Token![>]>()?;
    Ok(Trigger::Unexpected(ty))
}

fn parse_guard(input: ParseStream) -> Result<Option<GuardExpr>> {
    if !input.peek(syn::token::Bracket) {
        return Ok(None);
    }
    let content;
    syn::bracketed!(content in input);
    let expr: Expr = content.parse()?;
    GuardExpr::from_expr(expr).map(Some)
}

impl GuardExpr {
    fn from_expr(expr: Expr) -> Result<Self> {
        match expr {
            Expr::Path(_) | Expr::Closure(_) => Ok(Self::Leaf(expr)),
            Expr::Paren(paren) => Self::from_expr(*paren.expr),
            Expr::Unary(syn::ExprUnary {
                op: UnOp::Not(_),
                expr,
                ..
            }) => Ok(Self::Not(Box::new(Self::from_expr(*expr)?))),
            Expr::Binary(binary) => {
                let left = Box::new(Self::from_expr(*binary.left)?);
                let right = Box::new(Self::from_expr(*binary.right)?);
                match binary.op {
                    BinOp::And(_) => Ok(Self::And(left, right)),
                    BinOp::Or(_) => Ok(Self::Or(left, right)),
                    op => Err(Error::new(
                        op.span(),
                        "guards combine with `&&`, `||` and `!` only",
                    )),
                }
            }
            other => Err(Error::new(
                other.span(),
                "guard must be a function path or closure, combined with `&&`, `||`, `!`",
            )),
        }
    }
}

fn parse_actions(input: ParseStream) -> Result<Vec<Expr>> {
    if input.parse::<Option<Token![/]>>()?.is_none() {
        return Ok(Vec::new());
    }
    if input.peek(Token![|]) || input.peek(Token![move]) {
        return Err(input.error("wrap closure actions in parentheses: `/ (|ctx: &mut C| ...)`"));
    }
    if !input.peek(syn::token::Paren) {
        let path: syn::ExprPath = input.parse()?;
        return Ok(vec![Expr::Path(path)]);
    }
    let content;
    syn::parenthesized!(content in input);
    let items = Punctuated::<Expr, Token![,]>::parse_terminated(&content)?;
    let mut actions = Vec::new();
    for item in items {
        let (Expr::Path(_) | Expr::Closure(_)) = item else {
            return Err(Error::new(
                item.span(),
                "action must be a function path or closure",
            ));
        };
        actions.push(item);
    }
    Ok(actions)
}
