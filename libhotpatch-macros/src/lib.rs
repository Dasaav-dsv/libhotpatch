use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Abi, FnArg, ImplItemFn, LitByteStr, LitStr, Pat, PatWild, Token, parse_macro_input,
    parse_quote, token::Extern,
};

use crate::{args::Args, hotpatch_fn::HotpatchFn};

mod args;
mod hotpatch_fn;

#[proc_macro_attribute]
pub fn hotpatch(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let Args { is_checked } = parse_macro_input!(args as Args);
    let hotpatch_fn = parse_macro_input!(input as HotpatchFn);

    if is_checked {
        hotpatch_checked(hotpatch_fn)
    } else {
        hotpatch_unchecked(hotpatch_fn)
    }
    .into()
}

fn hotpatch_checked(HotpatchFn { inner, outer }: HotpatchFn) -> TokenStream {
    let ImplItemFn {
        attrs,
        vis,
        defaultness,
        sig,
        ..
    } = outer;

    let sig_str = quote!(sig).to_string();
    let sig_lit = LitByteStr::new(sig_str.as_bytes(), Span::call_site());

    let outer_fn = &sig.ident;
    let inner_fn = &inner.sig.ident;

    let args = inner.sig.inputs.iter().map(|input| match input {
        FnArg::Receiver(_) => parse_quote!(self),
        FnArg::Typed(typed) => fn_input_pat_to_ts(&typed.pat),
    });

    let tuple_args_outer = args.clone();
    let tuple_args_inner = args.clone();

    quote! {
        #(#attrs)*
        #vis #defaultness #sig {
            extern "C-unwind" fn checked_call(ptr: *const u8, len: usize) -> libhotpatch::BoxedSlice<u8> {
                #inner
                let (#(#tuple_args_inner,)*) = unsafe {
                    libhotpatch::rmp_serde::from_slice(::std::slice::from_raw_parts(ptr, len))
                        .expect("checked hot-patch input deserialization failed")
                };
                let output = unsafe {
                    libhotpatch::rmp_serde::to_vec_named(&#inner_fn(#(#args,)*))
                        .expect("checked hot-patch output serialization failed")
                };
                libhotpatch::BoxedSlice::new(&output)
            }
            fn type_of() -> (u128, &'static str) {
                let name = ::std::any::type_name_of_val(&#outer_fn);
                let mut hasher = libhotpatch::Xxh3::new();
                ::std::hash::Hash::hash(#sig_lit, &mut hasher);
                ::std::hash::Hash::hash(name.as_bytes(), &mut hasher);
                (hasher.digest128(), name)
            }
            #[libhotpatch::distributed_slice(libhotpatch::HOTPATCH_FN)]
            #[linkme(crate = libhotpatch::linkme)]
            static HOTPATCH_FN: (
                ::std::sync::atomic::AtomicPtr<()>,
                libhotpatch::LibraryHandle,
                fn() -> (u128, &'static str),
            ) = (
                ::std::sync::atomic::AtomicPtr::new(checked_call as *mut ()),
                libhotpatch::LibraryHandle::null(),
                type_of,
            );
            libhotpatch::Watcher::get().map(libhotpatch::Watcher::poll);
            let library_handle = HOTPATCH_FN.1.clone();
            let serialized = libhotpatch::rmp_serde::to_vec_named(&(#(#tuple_args_outer,)*))
                .expect("checked hot-patch input serialization failed");
            let serialized_output = unsafe {
                ::std::mem::transmute::<_, extern "C-unwind" fn(_, _) -> libhotpatch::BoxedSlice<u8>>(
                    HOTPATCH_FN.0.load(::std::sync::atomic::Ordering::Relaxed))
                        (serialized.as_ptr(), serialized.len())
            };
            libhotpatch::rmp_serde::from_slice(&serialized_output)
                .expect("checked hot-patch output deserialization failed")
        }
    }
}

fn hotpatch_unchecked(HotpatchFn { mut inner, outer }: HotpatchFn) -> TokenStream {
    let ImplItemFn {
        attrs,
        vis,
        defaultness,
        sig,
        ..
    } = outer;

    inner
        .attrs
        .push(parse_quote!(#[allow(improper_ctypes_definitions)]));

    let abi = inner
        .sig
        .abi
        .get_or_insert_with(|| Abi {
            extern_token: Extern(Span::call_site()),
            name: Some(LitStr::new("C-unwind", Span::call_site())),
        })
        .clone();

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
            fn type_of() -> (u128, &'static str) {
                let name = ::std::any::type_name_of_val(&#outer_fn);
                let mut hasher = libhotpatch::Xxh3::new();
                ::std::hash::Hash::hash(#sig_lit, &mut hasher);
                ::std::hash::Hash::hash(name.as_bytes(), &mut hasher);
                (hasher.digest128(), name)
            }
            #[libhotpatch::distributed_slice(libhotpatch::HOTPATCH_FN)]
            #[linkme(crate = libhotpatch::linkme)]
            static HOTPATCH_FN: (
                ::std::sync::atomic::AtomicPtr<()>,
                libhotpatch::LibraryHandle,
                fn() -> (u128, &'static str),
            ) = (
                ::std::sync::atomic::AtomicPtr::new(#inner_fn as *mut ()),
                libhotpatch::LibraryHandle::null(),
                type_of,
            );
            libhotpatch::Watcher::get().map(libhotpatch::Watcher::poll);
            let library_handle = HOTPATCH_FN.1.clone();
            unsafe {
                ::std::mem::transmute::<_, #abi fn(#(#wild,)*) -> _>(
                    HOTPATCH_FN.0.load(::std::sync::atomic::Ordering::Relaxed))
                        (#(#args,)*)
            }
        }
    }
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
