use syn::{
    Error, Ident, Result,
    parse::{Parse, ParseStream},
};

pub struct Args {
    pub is_checked: bool,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        if !input.peek(Ident) {
            return Ok(Args { is_checked: false });
        }

        let checked = input.parse::<Ident>()?;

        if checked.to_string() != "checked" {
            return Err(Error::new_spanned(
                &checked,
                "unsupported attribute, is not one of: \"checked\"",
            ));
        }

        if !cfg!(feature = "checked") {
            return Err(Error::new_spanned(
                &checked,
                "feature \"checked\" is disabled",
            ));
        }

        Ok(Args { is_checked: true })
    }
}
