use std::mem;

use quote::format_ident;
use syn::{
    Block, Error, FnArg, ImplItemFn, ItemFn, Result, Signature, Type, Visibility,
    parse::{Parse, ParseStream},
    token::Brace,
};

pub struct HotpatchFn {
    pub inner: ItemFn,
    pub outer: ImplItemFn,
}

impl Parse for HotpatchFn {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut outer = input.parse::<ImplItemFn>()?;

        if let Some(constness) = &outer.sig.constness {
            return Err(Error::new_spanned(
                constness,
                "a hot-patch function cannot be `const`",
            ));
        }

        if outer.sig.unsafety.is_none() {
            return Err(Error::new_spanned(
                &outer.sig,
                "a hot-patch function must be marked `unsafe`",
            ));
        }

        if let Some(abi) = &outer.sig.abi
            && let Some(abi_name) = &abi.name
            && abi_name.value() == "Rust"
        {
            return Err(Error::new_spanned(
                abi,
                "a hot-patch function cannot be `extern \"Rust\"`",
            ));
        }

        if let Some(recv) = outer.sig.receiver() {
            return Err(Error::new_spanned(
                recv,
                "a hot-patch function cannot use `Self`",
            ));
        }

        if outer.sig.generics.const_params().count() != 0
            || outer.sig.generics.type_params().count() != 0
        {
            return Err(Error::new_spanned(
                &outer.sig.generics,
                "a hot-patch function cannot use non-lifetime generics",
            ));
        }

        if let Some(impl_trait) = outer.sig.inputs.iter().find_map(|input| match input {
            FnArg::Typed(typed) => matches!(*typed.ty, Type::ImplTrait(_)).then_some(&*typed.ty),
            _ => None,
        }) {
            return Err(Error::new_spanned(
                impl_trait,
                "a hot-patch function cannot use `impl Trait` type parameters",
            ));
        }

        let brace_token = Brace(outer.block.brace_token.span);
        let block = mem::replace(
            &mut outer.block,
            Block {
                brace_token,
                stmts: vec![],
            },
        );

        let inner = ItemFn {
            attrs: outer.attrs.clone(),
            vis: Visibility::Inherited,
            sig: Signature {
                ident: format_ident!("{}_inner", outer.sig.ident),
                ..outer.sig.clone()
            },
            block: Box::new(block),
        };

        Ok(Self { inner, outer })
    }
}
