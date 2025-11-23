use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Abi, FnArg, ImplItemFn, LitByteStr, LitStr, Pat, PatWild, Token, parse_macro_input,
    parse_quote, token::Extern,
};

use crate::hotpatch_fn::HotpatchFn;

mod hotpatch_fn;

#[proc_macro_attribute]
pub fn hotpatch(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let HotpatchFn { inner, outer } = parse_macro_input!(input as HotpatchFn);

    let ImplItemFn {
        mut attrs,
        vis,
        defaultness,
        mut sig,
        ..
    } = outer;

    attrs.push(parse_quote!(#[allow(improper_ctypes_definitions)]));

    sig.abi.get_or_insert_with(|| Abi {
        extern_token: Extern(Span::call_site()),
        name: Some(LitStr::new("C-unwind", Span::call_site())),
    });

    let sig_str = quote!(sig).to_string();
    let sig_lit = LitByteStr::new(sig_str.as_bytes(), Span::call_site());

    let outer_fn = &sig.ident;
    let inner_fn = &inner.sig.ident;

    let args = inner.sig.inputs.iter().map(|input| match input {
        FnArg::Receiver(_) => parse_quote!(self),
        FnArg::Typed(typed) => fn_input_pat_to_ts(&typed.pat),
    });

    let wild = inner.sig.inputs.iter().map(|_| PatWild {
        attrs: vec![],
        underscore_token: Token![_](Span::call_site()),
    });

    quote! {
        #(#attrs)*
        #vis #defaultness #sig {
            #inner
            fn __libhotpatch_type_of() -> (u128, &'static str) {
                let name = ::std::any::type_name_of_val(&#outer_fn);
                let mut hasher = libhotpatch::Xxh3::new();
                ::std::hash::Hash::hash(#sig_lit, &mut hasher);
                ::std::hash::Hash::hash(name.as_bytes(), &mut hasher);
                (hasher.digest128(), name)
            }
            #[libhotpatch::distributed_slice(libhotpatch::HOTPATCH_FN)]
            #[linkme(crate = libhotpatch::linkme)]
            static __LIBHOTPATCH_HOTPATCH_FN: (
                ::std::sync::atomic::AtomicPtr<()>,
                libhotpatch::LibraryHandle,
                fn() -> (u128, &'static str),
            ) = (
                ::std::sync::atomic::AtomicPtr::new(#inner_fn as *mut ()),
                libhotpatch::LibraryHandle::null(),
                __libhotpatch_type_of,
            );
            libhotpatch::Watcher::get().map(libhotpatch::Watcher::poll);
            let __libhotpatch_library_handle = __LIBHOTPATCH_HOTPATCH_FN.1.clone();
            unsafe {
                ::std::mem::transmute::<_, fn(#(#wild,)*) -> _>(
                    __LIBHOTPATCH_HOTPATCH_FN.0.load(::std::sync::atomic::Ordering::Relaxed))(#(#args,)*
                )
            }
        }
    }
    .into()
}

fn fn_input_pat_to_ts(pat: &Pat) -> TokenStream {
    match pat {
        Pat::Ident(pat_ident) => pat_ident.ident.clone().to_token_stream(),
        Pat::Paren(pat_paren) => fn_input_pat_to_ts(&pat_paren.pat),
        Pat::Reference(pat_ref) => fn_input_pat_to_ts(&pat_ref.pat),
        Pat::Tuple(pat_tuple) => {
            let elems = &pat_tuple.elems;
            parse_quote!((#elems))
        }
        Pat::Struct(pat_struct) => {
            let path = &pat_struct.path;
            let members = pat_struct.fields.iter().map(|field_pat| &field_pat.member);

            parse_quote!(#path { #(#members,)* })
        }
        Pat::TupleStruct(pat_tstruct) => {
            let path = &pat_tstruct.path;
            let elems = pat_tstruct.elems.iter().map(fn_input_pat_to_ts);

            parse_quote!(#path(#(#elems,)*))
        }
        _ => panic!("unsupported type pattern in function input position"),
    }
}
