use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, ItemStruct, Path};

#[proc_macro_derive(WithPic)]
pub fn derive_with_pic(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;

    let gen = quote! {
        impl #name {
            pub async fn populate_pic_link_from<K: crate::traits::PicKey>(&mut self, asset_ops: &crate::db::AssetOperations, key_src: &K) {
                if key_src.has_pic() {
                    let url = asset_ops.get_object_presign(&key_src.pic_key()).await.ok();
                    self.pic_link = url;
                }
            }
        }
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn with_pic(attr: TokenStream, item: TokenStream) -> TokenStream {
    let from_path = parse_macro_input!(attr as Path);
    let item_struct = parse_macro_input!(item as ItemStruct);
    let target_ident = &item_struct.ident;

    // Build field initializers by name; special-case `pic_link` to None
    let mut inits = Vec::new();
    match &item_struct.fields {
        Fields::Named(named) => {
            for f in &named.named {
                let fname = f.ident.as_ref().expect("expected named field");
                if fname == "pic_link" {
                    inits.push(quote! { #fname: None });
                } else {
                    inits.push(quote! { #fname: src.#fname.clone() });
                }
            }
        }
        _ => panic!("#[with_pic] only supports structs with named fields"),
    }

    let gen = quote! {
        #item_struct

        impl From<& #from_path> for #target_ident {
            fn from(src: & #from_path) -> Self {
                Self { #(#inits),* }
            }
        }
    };

    gen.into()
}
