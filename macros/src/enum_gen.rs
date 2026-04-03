//! Code generation for #[derive(TraceEnum)].

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Token, Variant, punctuated::Punctuated};

/// Generate code for #[derive(TraceEnum)]
pub fn generate_trace_enum(ident: &Ident, variants: &[(Ident, Option<String>)]) -> TokenStream {
    let display_arms: Vec<TokenStream> = variants
        .iter()
        .map(|(var_ident, value)| {
            let binding = var_ident.to_string();
            let s = value.as_deref().unwrap_or(&binding);
            quote! { Self::#var_ident => f.write_str(#s) }
        })
        .collect();

    let from_str_arms: Vec<TokenStream> = variants
        .iter()
        .map(|(var_ident, value)| {
            let binding = var_ident.to_string();
            let s = value.as_deref().unwrap_or(&binding);
            quote! { #s => Ok(Self::#var_ident) }
        })
        .collect();

    let values: Vec<String> = variants
        .iter()
        .map(|(var_ident, value)| {
            value
                .as_deref()
                .unwrap_or(&var_ident.to_string())
                .to_string()
        })
        .collect();

    quote! {
        impl ::std::fmt::Display for #ident {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl ::std::str::FromStr for #ident {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms),*,
                    _ => Err(::std::format!("invalid {}: {}", stringify!(#ident), s)),
                }
            }
        }

        impl ::trace_parser::payload_template::TraceEnum for #ident {
            fn values() -> &'static [&'static str] {
                &[#(#values),*]
            }
        }
    }
}

/// Parse variants and #[value("...")] attributes
pub fn parse_variants(variants: &Punctuated<Variant, Token![,]>) -> Vec<(Ident, Option<String>)> {
    variants
        .iter()
        .map(|v| {
            let value = v
                .attrs
                .iter()
                .find(|a| a.path().is_ident("value"))
                .and_then(|a| a.parse_args::<syn::LitStr>().ok().map(|s| s.value()));
            (v.ident.clone(), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trace_enum_simple() {
        let name: Ident = syn::parse_quote!(TestState);
        let variants = vec![(syn::parse_quote!(A), None), (syn::parse_quote!(B), None)];

        let output = generate_trace_enum(&name, &variants);
        let s = output.to_string();

        assert!(s.contains("Display"));
        assert!(s.contains("FromStr"));
        assert!(s.contains("TraceEnum"));
    }

    #[test]
    fn test_generate_trace_enum_with_values() {
        let name: Ident = syn::parse_quote!(TestState);
        let variants = vec![
            (syn::parse_quote!(Sleeping), Some("S".to_string())),
            (syn::parse_quote!(Running), Some("R".to_string())),
        ];

        let output = generate_trace_enum(&name, &variants);
        let s = output.to_string();

        assert!(s.contains("S"));
        assert!(s.contains("R"));
    }

    #[test]
    fn test_parse_variants_from_syn() {
        let enum_item: syn::ItemEnum = syn::parse_quote! {
            enum Test { A, B, #[value("C")] CustomC }
        };
        let result = parse_variants(&enum_item.variants);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0.to_string(), "A");
        assert_eq!(result[0].1, None);
        assert_eq!(result[1].0.to_string(), "B");
        assert_eq!(result[2].0.to_string(), "CustomC");
        assert_eq!(result[2].1, Some("C".to_string()));
    }

    #[test]
    fn test_generate_trace_enum_prev_state() {
        // Real-world example: sched_switch prev_state
        let name: Ident = syn::parse_quote!(PrevState);
        let variants = vec![
            (syn::parse_quote!(Sleeping), Some("S".to_string())),
            (syn::parse_quote!(Running), Some("R".to_string())),
            (syn::parse_quote!(DiskSleep), Some("D".to_string())),
            (syn::parse_quote!(Dead), Some("X".to_string())),
            (syn::parse_quote!(Zombie), None),
        ];

        let output = generate_trace_enum(&name, &variants);
        let s = output.to_string();

        // Display impl
        assert!(s.contains("write_str (\"S\")"));
        assert!(s.contains("write_str (\"Zombie\")"));

        // FromStr impl
        assert!(s.contains("\"S\" => Ok (Self :: Sleeping)"));
        assert!(s.contains("\"Zombie\" => Ok (Self :: Zombie)"));

        // TraceEnum impl — generates values() method
        assert!(s.contains("TraceEn"));
        assert!(s.contains("values"));
        assert!(s.contains("S"));
        assert!(s.contains("R"));
        assert!(s.contains("D"));
        assert!(s.contains("X"));
        assert!(s.contains("Zombie"));
    }

    #[test]
    fn test_generate_trace_enum_empty() {
        let name: Ident = syn::parse_quote!(Empty);
        let variants = vec![];

        let output = generate_trace_enum(&name, &variants);
        let s = output.to_string();

        // Should still generate valid impls
        assert!(s.contains("Display"));
        assert!(s.contains("FromStr"));
        assert!(s.contains("TraceEnum"));
    }

    #[test]
    fn test_parse_variants_with_numeric_values() {
        let enum_item: syn::ItemEnum = syn::parse_quote! {
            enum Level { #[value("0")] Zero, #[value("1")] One }
        };
        let result = parse_variants(&enum_item.variants);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].1, Some("0".to_string()));
        assert_eq!(result[1].1, Some("1".to_string()));
    }
}
