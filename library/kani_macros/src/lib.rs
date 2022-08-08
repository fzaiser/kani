// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;
#[cfg(kani)]
use {
    quote::quote,
    syn::{parse_macro_input, ItemFn},
};

#[cfg(all(not(kani), not(test)))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    // Not-Kani, not-test means this code shouldn't exist, return nothing.
    TokenStream::new()
}

#[cfg(all(not(kani), test))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Leave the code intact, so it can be easily be edited in an IDE,
    // but outside Kani, this code is likely never called.
    let mut result = TokenStream::new();

    result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[allow(dead_code)]
    //     $item
    // )
}

/// Handles #[kani::proof] if #[cfg(kani)] is active
///
/// For synchronous functions, it becomes a Kani-internal attribute.
///
/// For async functions, it translates to a synchronous function that calls `kani::block_on`.
/// Specifically, it translates
/// ```ignore
/// #[kani::proof]
/// #[attribute]
/// pub async fn harness() { ... }
/// ```
/// to
/// ```ignore
/// #[kani::proof]
/// #[attribute]
/// pub fn harness() {
///   async fn harness() { ... }
///   kani::block_on(harness())
/// }
/// ```
/// which is then translated again as a asynchronous function.
#[cfg(kani)]
#[proc_macro_attribute]
pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(item as ItemFn);
    let attrs = fn_item.attrs;
    let vis = fn_item.vis;
    let sig = fn_item.sig;
    let body = fn_item.block;

    assert!(attr.is_empty(), "#[kani::proof] does not take any arguments for now");

    if sig.asyncness.is_none() {
        quote!(
            #[kanitool::proof]
            #[no_mangle]
            #(#attrs)*
            #vis #sig #body
        )
        .into()
    } else {
        assert!(
            sig.inputs.is_empty(),
            "#[kani::proof] cannot be applied to async functions that take inputs for now"
        );
        let mut modified_sig = sig.clone();
        modified_sig.asyncness = None;
        let fn_name = &sig.ident;
        quote!(
            #[kani::proof]
            #(#attrs)*
            #vis #modified_sig {
                #sig #body
                kani::block_on(#fn_name())
            }
        )
        .into()
    }
}

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn unwind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not kani, we should leave the function alone
    item
}

/// Set Loop unwind limit for proof harnesses
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[cfg(kani)]
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Translate #[kani::unwind(arg)] to #[kanitool::unwind(arg)]
    let insert_string = "#[kanitool::unwind(".to_owned() + &attr.to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}
