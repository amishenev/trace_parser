//! Attribute parsing for trace_event macros.

use syn::{parse::{Parse, ParseStream}, Attribute, Ident, LitStr, Result, Token};

/// Parsed `#[trace_event(name = "...", aliases = ["...", ...])]` attribute
#[derive(Debug, Clone)]
pub struct TraceEventAttr {
    pub name: String,
    pub aliases: Vec<String>,
}

impl Parse for TraceEventAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = None;
        let mut aliases = Vec::new();

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
            }

            // Parse optional comma
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name.ok_or_else(|| syn::Error::new(input.span(), "missing 'name' attribute"))?;

        Ok(Self { name, aliases })
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

/// Parsed `#[define_template("...")]` attribute
#[derive(Debug, Clone)]
pub struct DefineTemplateAttr(pub String);

impl Parse for DefineTemplateAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let template: LitStr = input.parse()?;
        Ok(Self(template.value()))
    }
}

/// Parsed `#[field(ty = "...", name = "...", optional)]` attribute
#[derive(Debug, Clone)]
pub struct FieldAttr {
    pub ty: String,
    pub name: Option<String>,
    pub optional: bool,
}

impl Parse for FieldAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut ty = None;
        let mut name = None;
        let mut optional = false;

        // Parse comma-separated key-value pairs
        while !input.is_empty() {
            let key: Ident = input.parse()?;

            if key == "optional" {
                optional = true;
            } else {
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;

                if key == "ty" {
                    ty = Some(value.value());
                } else if key == "name" {
                    name = Some(value.value());
                }
            }

            // Parse optional comma
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        let ty = ty.ok_or_else(|| syn::Error::new(input.span(), "missing 'ty' attribute"))?;

        Ok(Self { ty, name, optional })
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
        .and_then(|attr| attr.parse_args().ok())
}
