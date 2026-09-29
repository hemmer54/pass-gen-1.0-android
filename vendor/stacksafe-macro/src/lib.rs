//! Procedural macro implementation for the `stacksafe` crate.

use proc_macro2::TokenStream;
use proc_macro2_diagnostics::{Diagnostic, SpanDiagnosticExt};
use quote::quote;
use quote::ToTokens;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::Item;
use syn::Path;
use syn::ReturnType;
use syn::Type;

#[proc_macro_attribute]
pub fn stacksafe(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = TokenStream::from(args);
    let item = TokenStream::from(item);
    match stacksafe_impl(args, item) {
        Ok(tokens) => tokens.into(),
        Err(diagnostic) => diagnostic.emit_as_item_tokens().into(),
    }
}

fn stacksafe_impl(args: TokenStream, item: TokenStream) -> Result<TokenStream, Diagnostic> {
    let mut crate_path: Option<Path> = None;
    let arg_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("crate") {
            if crate_path.is_some() {
                return Err(meta.error("duplicate attribute parameter `crate`"));
            }
            crate_path = Some(meta.value()?.parse()?);
            Ok(())
        } else {
            Err(meta.error(format!(
                "unknown attribute parameter `{}`",
                meta.path.to_token_stream()
            )))
        }
    });
    syn::parse::Parser::parse2(arg_parser, args)?;

    let mut item_fn = match syn::parse2::<Item>(item)? {
        Item::Fn(item_fn) => item_fn,
        item => {
            return Err(item
                .span()
                .error("#[stacksafe] can only be applied to functions"));
        }
    };

    if item_fn.sig.asyncness.is_some() {
        return Err(item_fn
            .sig
            .asyncness
            .span()
            .error("#[stacksafe] does not support async functions"));
    }

    let ret = match &item_fn.sig.output {
        ReturnType::Type(_, ty) if matches!(**ty, Type::ImplTrait(_)) => None,
        ret => Some(ret),
    };

    let stacksafe_crate = crate_path.unwrap_or_else(|| parse_quote!(::stacksafe));
    let block = &item_fn.block;
    let wrapped_block = quote! {
        {
            #stacksafe_crate::internal::stacker::maybe_grow(
                #stacksafe_crate::get_minimum_stack_size(),
                #stacksafe_crate::get_stack_allocation_size(),
                #stacksafe_crate::internal::with_protected(move || #ret { #block })
            )
        }
    };

    *item_fn.block = syn::parse2(wrapped_block)?;
    Ok(item_fn.into_token_stream())
}
