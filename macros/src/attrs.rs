//! Attribute parsing for trace_event macros.

use syn::{parse::{Parse, ParseStream}, Attribute, Ident, LitStr, Result, Token};

/// Parsed `#[trace_event(name = "...", aliases = ["...", ...])]` attribute
#[derive(Debug, Clone)]
pub struct TraceEventAttr {
    pub name: String,
    pub aliases: Vec<String>,
    pub generate_pymethods: bool,
}

impl Parse for TraceEventAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut aliases = Vec::new();
        let mut generate_pymethods = true;

        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if key == "name" {
                let value: LitStr = input.parse()?;
                name = Some(value.value());
            } else if key == "aliases" {
                // Parse array: ["alias1", "alias2"]
                let content;
                syn::bracketed!(content in input);
                let list = content.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])?;
                aliases = list.iter().map(|s| s.value()).collect();
            } else if key == "generate_pymethods" {
                let value: syn::LitBool = input.parse()?;
                generate_pymethods = value.value();
            }

            // Parse optional comma
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| syn::Error::new(input.span(), "missing 'name' attribute"))?;

        Ok(Self { name, aliases, generate_pymethods })
    }
}

/// Parsed `#[trace_markers("...", "...")]` attribute
#[derive(Debug, Clone)]
pub struct TraceMarkersAttr(pub Vec<String>);

impl Parse for TraceMarkersAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let list = input.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])?;
        let markers = list.iter().map(|s| s.value()).collect();
        Ok(Self(markers))
    }
}

/// Parsed `#[define_template("...", id = N)]` attribute
///
/// Template attributes with optional explicit id:
/// - `id`: Explicit format id (0, 1, 2, ...). Auto-assigned if not specified.
#[derive(Debug, Clone)]
pub struct DefineTemplateAttr {
    pub template: String,
    pub id: Option<u8>,
}

impl Parse for DefineTemplateAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let template: LitStr = input.parse()?;
        
        let id = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            // Check for `id = N`
            if input.peek(Ident) {
                let key: Ident = input.parse()?;
                if key == "id" {
                    input.parse::<Token![=]>()?;
                    let value: syn::LitInt = input.parse()?;
                    Some(value.base10_parse::<u8>()?)
                } else {
                    return Err(syn::Error::new(key.span(), "expected 'id'"));
                }
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Self { template: template.value(), id })
    }
}

/// Parsed `#[field(name = "...", regex = "...", optional, readonly, private)]` attribute
///
/// Field attributes control how struct fields are exposed to Python.
/// Type is inferred from the Rust field type (String, u32, i32, f64, bool).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FieldAttr {
    pub name: Option<String>,
    pub regex: Option<String>,
    pub optional: bool,
    pub readonly: bool,
    pub private: bool,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut regex = None;
        let mut optional = false;
        let mut readonly = false;
        let mut private = false;

        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.parse()?;

            if key == "optional" || key == "readonly" || key == "private" {
                if key == "optional" { optional = true; }
                else if key == "readonly" { readonly = true; }
                else if key == "private" { private = true; }
            } else {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                if key == "name" {
                    name = Some(value.value());
                } else if key == "regex" {
                    regex = Some(value.value());
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { name, regex, optional, readonly, private })
    }
}

/// Extract `#[trace_event(...)]` attribute from a list of attributes
pub fn find_trace_event_attr(attrs: &[Attribute]) -> Option<TraceEventAttr> {
    attrs.iter()
        .find(|attr| attr.path().is_ident("trace_event"))
        .and_then(|attr| attr.parse_args().ok())
}

/// Extract `#[trace_markers(...)]` attribute from a list of attributes
pub fn find_trace_markers_attr(attrs: &[Attribute]) -> Option<TraceMarkersAttr> {
    attrs.iter()
        .find(|attr| attr.path().is_ident("trace_markers"))
        .and_then(|attr| attr.parse_args().ok())
}

/// Extract all `#[define_template(...)]` attributes from a list of attributes
pub fn find_define_template_attrs(attrs: &[Attribute]) -> Vec<DefineTemplateAttr> {
    attrs.iter()
        .filter(|attr| attr.path().is_ident("define_template"))
        .filter_map(|attr| attr.parse_args().ok())
        .collect()
}

/// Extract `#[field(...)]` attribute from a field's attributes
pub fn find_field_attr(attrs: &[Attribute]) -> Option<FieldAttr> {
    attrs.iter()
        .find(|attr| attr.path().is_ident("field"))
        .and_then(|attr| {
            // #[field] без аргументов → пустой FieldAttr
            let empty = match &attr.meta {
                syn::Meta::Path(_) => true,
                syn::Meta::List(list) => list.tokens.is_empty(),
                syn::Meta::NameValue(_) => false,
            };
            if empty {
                Some(FieldAttr {
                    name: None,
                    regex: None,
                    optional: false,
                    readonly: false,
                    private: false,
                })
            } else {
                attr.parse_args().ok()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_trace_event_attr_basic() {
        let tokens = quote! { name = "sched_switch" };
        let attr: TraceEventAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, "sched_switch");
        assert!(attr.aliases.is_empty());
    }

    #[test]
    fn test_trace_event_attr_with_aliases() {
        let tokens = quote! { name = "sched_switch", aliases = ["sched_sw", "switch"] };
        let attr: TraceEventAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, "sched_switch");
        assert_eq!(attr.aliases, vec!["sched_sw", "switch"]);
    }

    #[test]
    fn test_trace_markers_attr() {
        let tokens = quote! { "B|", "ReceiveVsync" };
        let attr: TraceMarkersAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.0, vec!["B|", "ReceiveVsync"]);
    }

    #[test]
    fn test_define_template_attr() {
        let tokens = quote! { "prev_comm={prev_comm} prev_pid={prev_pid}" };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "prev_comm={prev_comm} prev_pid={prev_pid}");
        assert!(attr.id.is_none());
    }

    #[test]
    fn test_define_template_attr_with_id() {
        let tokens = quote! { "prev_comm={prev_comm}", id = 1 };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "prev_comm={prev_comm}");
        assert_eq!(attr.id, Some(1));
    }

    #[test]
    fn test_define_template_attr_with_id_zero() {
        let tokens = quote! { "value={value}", id = 0 };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "value={value}");
        assert_eq!(attr.id, Some(0));
    }

    #[test]
    fn test_field_attr_basic() {
        let tokens = quote! {};
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert!(attr.name.is_none());
        assert!(!attr.optional);
        assert!(!attr.readonly);
        assert!(!attr.private);
    }

    #[test]
    fn test_field_attr_with_name() {
        let tokens = quote! { name = "state" };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, Some("state".to_string()));
    }

    #[test]
    fn test_field_attr_optional() {
        let tokens = quote! { optional };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert!(attr.optional);
    }

    #[test]
    fn test_field_attr_readonly() {
        let tokens = quote! { readonly };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert!(attr.readonly);
    }

    #[test]
    fn test_field_attr_private() {
        let tokens = quote! { private };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert!(attr.private);
    }

    #[test]
    fn test_field_attr_all_flags() {
        let tokens = quote! { name = "state", optional, readonly, private };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, Some("state".to_string()));
        assert!(attr.optional);
        assert!(attr.readonly);
        assert!(attr.private);
    }
}
