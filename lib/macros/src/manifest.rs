use indexmap::IndexMap;
use syn::{
    Expr, Ident, LitStr, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
};

pub struct Manifest {
    pub machine: Expr,
    pub address_spaces: IndexMap<Ident, Expr>,
    pub components: IndexMap<LitStr, (Ident, Expr)>,
}

impl Parse for Manifest {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut machine = None;
        let mut address_spaces = IndexMap::new();
        let mut components = IndexMap::new();

        // Parse the fields in any order
        while !input.is_empty() {
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?; // Parse the colon

            match field_name.to_string().as_str() {
                "machine" => {
                    machine = Some(input.parse()?);
                }
                "address_spaces" => {
                    let content;
                    braced!(content in input);

                    while !content.is_empty() {
                        let key = content.parse()?;
                        content.parse::<Token![:]>()?;

                        let value = content.parse()?;

                        address_spaces.insert(key, value);

                        // Parse a comma if there are more fields
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                "components" => {
                    let content;
                    braced!(content in input);

                    while !content.is_empty() {
                        let component_ident = content.parse()?;

                        let manifest_name = {
                            let string_content;
                            parenthesized!(string_content in content);
                            string_content.parse::<LitStr>()?
                        };
                        content.parse::<Token![:]>()?;

                        let config = content.parse()?;

                        components.insert(manifest_name, (component_ident, config));

                        // Parse a comma if there are more fields
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        field_name,
                        "Unknown field in manifest. Expected 'machine', 'address_spaces', or 'components'.",
                    ));
                }
            }

            // Parse a comma if there are more fields
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Manifest {
            machine: machine.ok_or_else(|| {
                syn::Error::new(input.span(), "Missing 'machine' field in manifest")
            })?,
            address_spaces,
            components,
        })
    }
}
