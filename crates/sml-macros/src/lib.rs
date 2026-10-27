//! Proc macro behind [`sml::sml!`](https://docs.rs/sml). Use the `sml` crate instead.

mod ast;
mod codegen;
#[cfg(feature = "diagram")]
mod diagram;
mod model;

use proc_macro::TokenStream;

/// Defines a state machine from a Boost.SML-style transition table.
///
/// See the `sml` crate documentation for the syntax and semantics.
#[proc_macro]
pub fn sml(input: TokenStream) -> TokenStream {
    let def = syn::parse_macro_input!(input as ast::MachineDef);
    match expand(def) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(def: ast::MachineDef) -> syn::Result<proc_macro2::TokenStream> {
    let model = model::build(def)?;
    #[cfg(feature = "diagram")]
    {
        let docs = diagram::doc_lines(&model)?;
        let mut tokens = codegen::generate(&model, &docs);
        tokens.extend(diagram::diagram_impl(&model));
        Ok(tokens)
    }
    #[cfg(not(feature = "diagram"))]
    Ok(codegen::generate(&model, &[]))
}
