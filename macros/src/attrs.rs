//! Attribute parsing for trace_event macros.

use syn::{parse::{Parse, ParseStream}, ext::IdentExt, Attribute, Ident, LitStr, Result, Token};

/// Tracing mark type: begin or end marker
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkType {
    Begin,
    End,
}

/// Parsed `#[trace_event(name = "...", aliases = ["...", ...], skip_registration, begin, end)]` attribute
#[derive(Debug, Clone)]
pub struct TraceEventAttr {
    pub name: String,
    pub aliases: Vec<String>,
    pub generate_pymethods: bool,
    /// Skip automatic registration — used for events handled explicitly (e.g. TraceMarkBegin/End).
    pub skip_registration: bool,
    /// This is a tracing_mark begin/end marker — used by TracingMarkEvent derive.
    pub mark_type: Option<MarkType>,
}

impl Parse for TraceEventAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut aliases = Vec::new();
        let mut generate_pymethods = true;
        let mut skip_registration = false;
        let mut mark_type = None;

        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.call(Ident::parse_any)?;

            if key == "skip_registration" {
                skip_registration = true;
            } else if key == "begin" {
                mark_type = Some(MarkType::Begin);
            } else if key == "end" {
                mark_type = Some(MarkType::End);
            } else if key == "name" {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                name = Some(value.value());
            } else if key == "aliases" {
                input.parse::<Token![=]>()?;
                // Parse array: ["alias1", "alias2"]
                let content;
                syn::bracketed!(content in input);
                let list = content.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])?;
                aliases = list.iter().map(|s| s.value()).collect();
            } else if key == "generate_pymethods" {
                input.parse::<Token![=]>()?;
                let value: syn::LitBool = input.parse()?;
                generate_pymethods = value.value();
            } else {
                return Err(syn::Error::new(key.span(), "unknown attribute"));
            }

            // Parse optional comma
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| syn::Error::new(input.span(), "missing 'name' attribute"))?;

        Ok(Self { name, aliases, generate_pymethods, skip_registration, mark_type })
    }
}

/// Parsed `#[fast_match(contains_any = ["...", ...])]` — `FastMatch::payload_quick_check` via `contains_any`.
#[derive(Debug, Clone)]
pub struct FastMatchAttr {
    pub contains_any: Vec<String>,
}

impl Parse for FastMatchAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut contains_any = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if key == "contains_any" {
                let content;
                syn::bracketed!(content in input);
                let list = content.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])?;
                contains_any = list.iter().map(|s| s.value()).collect();
            } else {
                return Err(syn::Error::new(key.span(), "expected contains_any"));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { contains_any })
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

/// Parsed `#[define_template("...", id = N, detect = ["..."], field_name = "...")]` attribute
///
/// Template attributes with optional explicit id, detect markers, and extra field definitions.
/// - `id`: Explicit format id (0, 1, 2, ...). Auto-assigned if not specified.
/// - `detect`: Substring markers for format detection via SIMD.
/// - Extra fields (e.g., `extra_info = r"\[[^\]]+\]"`): Field specs for ignored fields.
#[derive(Debug, Clone)]
pub struct DefineTemplateAttr {
    pub template: String,
    pub id: Option<u8>,
    pub detect: Vec<String>,
    /// Extra fields defined in the attribute (e.g., ignored fields).
    /// Maps field_name -> regex
    pub extra_fields: Vec<(String, String)>,
}

impl Parse for DefineTemplateAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let template: LitStr = input.parse()?;

        let mut id: Option<u8> = None;
        let mut detect: Vec<String> = Vec::new();
        let mut extra_fields: Vec<(String, String)> = Vec::new();

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: Ident = input.call(Ident::parse_any)?;
            if key == "id" {
                input.parse::<Token![=]>()?;
                let value: syn::LitInt = input.parse()?;
                id = Some(value.base10_parse::<u8>()?);
            } else if key == "detect" {
                input.parse::<Token![=]>()?;
                let content;
                syn::bracketed!(content in input);
                let list = content.parse_terminated(|input: ParseStream| input.parse::<LitStr>(), Token![,])?;
                detect = list.iter().map(|s| s.value()).collect();
            } else {
                // Assume it's an extra field: key = "regex"
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                extra_fields.push((key.to_string(), value.value()));
            }
        }

        Ok(Self { template: template.value(), id, detect, extra_fields })
    }
}

/// Parsed `#[field(name = "...", regex = "...", choice = ["a", "b"], format = "...", readonly, private)]` attribute
///
/// Field attributes control how struct fields are exposed to Python.
/// Type is inferred from the Rust field type (String, u32, i32, f64, bool).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FieldAttr {
    pub name: Option<String>,
    pub regex: Option<String>,
    pub choice: Vec<String>,
    /// Custom format string for rendering (e.g. `"{:03}"`).
    pub format: Option<String>,
    pub readonly: bool,
    pub private: bool,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut regex = None;
        let mut choice = Vec::new();
        let mut format = None;
        let mut readonly = false;
        let mut private = false;

        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.call(Ident::parse_any)?;

            if key == "format" {
                let _: Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                format = Some(value.value());
            } else if key == "choice" {
                // Parse array: ["val1", "val2"] or [11, 12]
                input.parse::<Token![=]>()?;
                let content;
                syn::bracketed!(content in input);
                let list = content.parse_terminated(|input: ParseStream| input.parse::<syn::Lit>(), Token![,])?;
                for lit in list {
                    match lit {
                        syn::Lit::Str(s) => choice.push(s.value()),
                        syn::Lit::Int(i) => choice.push(i.base10_digits().to_string()),
                        _ => {}
                    }
                }
            } else if key == "readonly" || key == "private" {
                if key == "readonly" { readonly = true; }
                else if key == "private" { private = true; }
            } else {
                input.parse::<Token![=]>()?;

                if key == "name" {
                    let value: LitStr = input.parse()?;
                    name = Some(value.value());
                } else if key == "regex" {
                    let value: LitStr = input.parse()?;
                    regex = Some(value.value());
                }
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { name, regex, choice, readonly, private, format })
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

/// Extract `#[fast_match(...)]` attribute from a list of attributes
pub fn find_fast_match_attr(attrs: &[Attribute]) -> Option<FastMatchAttr> {
    attrs.iter()
        .find(|attr| attr.path().is_ident("fast_match"))
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
                    choice: Vec::new(),
                    format: None,
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
    fn test_trace_event_attr_skip_registration() {
        let tokens = quote! { name = "tracing_mark_write", skip_registration };
        let attr: TraceEventAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, "tracing_mark_write");
        assert!(attr.skip_registration);
    }

    #[test]
    fn test_trace_event_attr_default_no_skip_registration() {
        let tokens = quote! { name = "sched_switch" };
        let attr: TraceEventAttr = syn::parse2(tokens).unwrap();
        assert!(!attr.skip_registration);
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
        assert!(attr.detect.is_empty());
    }

    #[test]
    fn test_define_template_attr_with_id() {
        let tokens = quote! { "prev_comm={prev_comm}", id = 1 };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "prev_comm={prev_comm}");
        assert_eq!(attr.id, Some(1));
    }

    #[test]
    fn test_define_template_attr_with_extra_fields() {
        let tokens = quote! { "{?ignore:extra_info}ReceiveVsync {frame}", extra_info = r"\[[^\]]+\]" };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "{?ignore:extra_info}ReceiveVsync {frame}");
        assert_eq!(attr.extra_fields.len(), 1);
        assert_eq!(attr.extra_fields[0].0, "extra_info");
        assert_eq!(attr.extra_fields[0].1, r"\[[^\]]+\]");
    }

    #[test]
    fn test_define_template_attr_with_id_zero() {
        let tokens = quote! { "value={value}", id = 0 };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "value={value}");
        assert_eq!(attr.id, Some(0));
    }

    #[test]
    fn test_define_template_attr_with_detect() {
        let tokens = quote! { "comm={comm} reason={reason}", detect = ["reason="] };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "comm={comm} reason={reason}");
        assert_eq!(attr.detect, vec!["reason="]);
    }

    #[test]
    fn test_define_template_attr_with_id_and_detect() {
        let tokens = quote! { "comm={comm} reason={reason}", id = 1, detect = ["reason="] };
        let attr: DefineTemplateAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.template, "comm={comm} reason={reason}");
        assert_eq!(attr.id, Some(1));
        assert_eq!(attr.detect, vec!["reason="]);
    }

    #[test]
    fn test_field_attr_basic() {
        let tokens = quote! {};
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert!(attr.name.is_none());
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
    fn test_field_attr_format() {
        let tokens = quote! { format = "{:03}" };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.format, Some("{:03}".to_string()));
    }

    #[test]
    fn test_field_attr_with_regex() {
        let tokens = quote! { regex = r"\d{3}" };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.regex, Some(r"\d{3}".to_string()));
    }

    #[test]
    fn test_field_attr_with_choice() {
        let tokens = quote! { choice = ["ddr_devfreq", "l3c_devfreq"] };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.choice, vec!["ddr_devfreq", "l3c_devfreq"]);
    }

    #[test]
    fn test_trace_event_attr_generate_pymethods() {
        let tokens = quote! { name = "sched_switch", generate_pymethods = false };
        let attr: TraceEventAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, "sched_switch");
        assert!(!attr.generate_pymethods);
    }

    #[test]
    fn test_field_attr_all_flags() {
        let tokens = quote! { name = "state", readonly, private };
        let attr: FieldAttr = syn::parse2(tokens).unwrap();
        assert_eq!(attr.name, Some("state".to_string()));
        assert!(attr.readonly);
        assert!(attr.private);
    }
}
