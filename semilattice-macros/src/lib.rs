use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Index};

#[proc_macro_derive(SemiLattice)]
pub fn derive_semilattice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    for param in &mut input.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(semilattice::SemiLattice));
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let join = semilattice_join(&input.data);

    quote!(
        impl #impl_generics semilattice::SemiLattice for #name #ty_generics #where_clause {
            fn join(self, other: Self) -> Self {
                #join
            }
        }
    )
    .into()
}

fn semilattice_join(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() =>
                        #name: semilattice::SemiLattice::join(self.#name, other.#name),
                    }
                });
                quote! {
                    Self {
                        #(#fields)*
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let fields = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() =>
                        semilattice::SemiLattice::join(self.#index, other.#index),
                    }
                });
                quote! {
                    Self(#(#fields)*)
                }
            }
            Fields::Unit => {
                quote!(Self)
            }
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}

#[proc_macro_derive(SemiLatticeOrd)]
pub fn derive_semilattice_ord(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    for param in &mut input.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(semilattice::SemiLattice));
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let partial_cmp = partial_ord_cmp(&input.data);

    quote!(
        impl #impl_generics core::cmp::PartialOrd for #name #ty_generics #where_clause {
            fn partial_cmp(&self, other: &Self) -> core::option::Option<core::cmp::Ordering> {
                use core::cmp::PartialOrd;
                #partial_cmp
            }
        }
    )
    .into()
}

fn partial_ord_cmp(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let orders = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! { f.span() =>
                        PartialOrd::partial_cmp(&self.#name, &other.#name),
                    }
                });
                quote! {
                    semilattice::partial_ord_helper([#(#orders)*])
                }
            }
            Fields::Unnamed(ref fields) => {
                let orders = fields.unnamed.iter().enumerate().map(|(i, f)| {
                    let index = Index::from(i);
                    quote_spanned! { f.span() =>
                        PartialOrd::partial_cmp(&self.#index, &other.#index),
                    }
                });
                quote! {
                    semilattice::partial_ord_helper([#(#orders)*])
                }
            }
            Fields::Unit => {
                quote!(core::option::Option::Some(core::cmp::Ordering::Equal))
            }
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
