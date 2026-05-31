use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(ColorGetters)]
pub fn derive_color_getters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(f) => &f.named,
            _ => panic!("ColorGetters only works in named fields"),
        },
        _ => panic!("ColorGetters only works in structs"),
    };

    let getters = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        quote! {
             pub const fn #field_name(self) -> ratatui::style::Color {
                 self.#field_name.to_color()
             }
        }
    });
    quote! {
        impl #name {
            #(#getters)*
        }
    }
    .into()
}
