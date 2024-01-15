extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    try_test(attr, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn try_test(attr: TokenStream, input: ItemFn) -> syn::Result<Tokens> {
    let inner_test = if attr.is_empty() {
        quote! { std::prelude::v1::test }
    } else {
        attr.into()
    };

    let ItemFn {
        attrs: _attrs,
        vis,
        sig,
        block,
    } = input;

    let init_tracing = quote! {
        crate::INIT.call_once(|| tracing_subscriber::fmt()
            .compact()
            //.with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
            .with_env_filter("zip_static_handler=trace")
            .without_time()
            .with_line_number(false)
            .with_file(false)
            .try_init()
            .expect("could not init tracing subscriber")
        )
        // tracing_subscriber::fmt()
        //     .compact()
        //     .with_env_filter("acme_tls_alpn_01=trace")
        //     .without_time()
        //     .with_line_number(false)
        //     .try_init()
        //     .expect("could not init env filter");
    };

    let result = quote! {
      #[#inner_test]
      #vis #sig {
        mod init_test_tracing {
          pub fn init() {
            #init_tracing
          }
        }
        init_test_tracing::init();
        #block
      }
    };
    Ok(result)
}
