//! Python API generation for trace_event macros.

use crate::attrs::FieldAttr;
use crate::generator::{InferredType, is_option_type};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

/// Generate the complete `#[pymethods]` block for the struct
pub fn generate_pymethods_block(
    struct_name: &Ident,
    fields: &[(Ident, Type, FieldAttr)],
) -> TokenStream {
    let field_accessors = generate_field_accessors(fields);
    let new_fn = generate_new(fields);
    let repr_fn = generate_repr();
    let eq_fn = generate_eq(struct_name, fields);
    let str_fn = generate_str();
    let can_be_parsed_fn = generate_can_be_parsed();
    let parse_fn = generate_parse();
    let to_string_fn = generate_to_string();
    let unknown_thread_fn = generate_has_unknown_thread();
    let copy_fn = generate_copy();
    let deepcopy_fn = generate_deepcopy();
    let payload_fn = generate_payload();
    let template_fn = generate_template();

    quote! {
        #[pyo3::pymethods]
        impl #struct_name {
            #(#field_accessors)*
            #new_fn
            #repr_fn
            #eq_fn
            #str_fn
            #can_be_parsed_fn
            #parse_fn
            #to_string_fn
            #unknown_thread_fn
            #copy_fn
            #deepcopy_fn
            #payload_fn
            #template_fn

            #[getter]
            fn timestamp_ms(&self) -> f64 {
                self.timestamp * 1_000.0
            }

            #[setter]
            fn set_timestamp_ms(&mut self, value: f64) -> ::pyo3::PyResult<()> {
                ::trace_parser::common::validate_timestamp(value / 1_000.0)?;
                self.timestamp = value / 1_000.0;
                Ok(())
            }

            #[getter]
            fn timestamp_ns(&self) -> u64 {
                (self.timestamp * 1_000_000_000.0).round() as u64
            }

            #[setter]
            fn set_timestamp_ns(&mut self, value: u64) -> ::pyo3::PyResult<()> {
                self.timestamp = (value as f64) / 1_000_000_000.0;
                Ok(())
            }
        }
    }
}

fn generate_field_accessors(fields: &[(Ident, Type, FieldAttr)]) -> Vec<TokenStream> {
    fields
        .iter()
        .filter(|(_, _, field_attr)| !field_attr.private)
        .flat_map(|(field_name, field_ty, field_attr)| {
            let setter_name = syn::Ident::new(&format!("set_{}", field_name), field_name.span());

            let getter = quote! {
                #[getter]
                fn #field_name(&self) -> #field_ty {
                    self.#field_name.clone()
                }
            };

            if field_attr.readonly {
                vec![getter]
            } else {
                let setter = quote! {
                    #[setter]
                    fn #setter_name(&mut self, value: #field_ty) {
                        self.#field_name = value;
                    }
                };
                vec![getter, setter]
            }
        })
        .collect()
}

/// Generate `#[new]` constructor
fn generate_new(fields: &[(Ident, Type, FieldAttr)]) -> TokenStream {
    let field_names: Vec<&Ident> = fields.iter().map(|(name, _, _)| name).collect();

    let field_params: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, field_ty, _field_attr)| {
            let inferred =
                InferredType::from_syn(field_ty).expect("unsupported field type in constructor");
            let ty = inferred.rust_type_tokens();

            if is_option_type(field_ty) {
                quote! { #field_name: ::std::option::Option<#ty> }
            } else {
                quote! { #field_name: #ty }
            }
        })
        .collect();

    let field_inits: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, _field_ty, _field_attr)| {
            quote! { #field_name }
        })
        .collect();

    quote! {
        #[new]
        #[pyo3(signature = (#(#field_names),*))]
        #[allow(clippy::too_many_arguments)]
        fn new(#(#field_params),*) -> ::pyo3::PyResult<Self> {
            ::trace_parser::common::validate_timestamp(timestamp)?;
            Ok(Self {
                #(#field_inits),*
            })
        }
    }
}

/// Generate `__repr__` method
fn generate_repr() -> TokenStream {
    quote! {
        fn __repr__(&self) -> ::pyo3::PyResult<::std::string::String> {
            Ok(format!("{:?}", self))
        }
    }
}

/// Generate `__eq__` method
fn generate_eq(struct_name: &Ident, fields: &[(Ident, Type, FieldAttr)]) -> TokenStream {
    let field_comparisons: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, _, _)| {
            quote! { self.#field_name == other.#field_name }
        })
        .collect();

    quote! {
        fn __eq__(&self, other: &#struct_name) -> bool {
            #(#field_comparisons)&&*
        }
    }
}

/// Generate `__str__` method (delegates to to_string)
fn generate_str() -> TokenStream {
    quote! {
        fn __str__(&self) -> ::pyo3::PyResult<::std::string::String> {
            self.to_string()
        }
    }
}

/// Generate `can_be_parsed` static method
fn generate_can_be_parsed() -> TokenStream {
    quote! {
        #[staticmethod]
        pub fn can_be_parsed(line: &str) -> bool {
            <Self as ::trace_parser::common::FastMatch>::quick_check(line)
        }
    }
}

/// Generate `parse` static method
fn generate_parse() -> TokenStream {
    quote! {
        #[staticmethod]
        pub fn parse(line: &str) -> ::std::option::Option<Self> {
            if !Self::can_be_parsed(line) {
                return None;
            }
            ::trace_parser::common::parse_template_event::<Self>(line)
        }
    }
}

/// Generate `to_string` method
fn generate_to_string() -> TokenStream {
    quote! {
        pub fn to_string(&self) -> ::pyo3::PyResult<::std::string::String> {
            ::trace_parser::common::validate_timestamp(self.timestamp)?;
            let payload = <Self as ::trace_parser::common::TemplateEvent>::render_payload(self)?;
            Ok(::trace_parser::trace::format_trace_header(
                &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
                &self.flags, self.timestamp, &self.event_name,
                &payload
            ))
        }
    }
}

/// Generate helper to detect unknown thread placeholder `<...>`
fn generate_has_unknown_thread() -> TokenStream {
    quote! {
        pub fn has_unknown_thread(&self) -> bool {
            self.thread_name.trim() == "<...>"
        }
    }
}

/// Generate `__copy__` method
fn generate_copy() -> TokenStream {
    quote! {
        fn __copy__(slf: ::pyo3::PyRef<'_, Self>, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<::pyo3::Py<Self>> {
            Ok(::pyo3::IntoPyObject::into_pyobject(slf.clone(), py)?.unbind())
        }
    }
}

/// Generate `__deepcopy__` method
fn generate_deepcopy() -> TokenStream {
    quote! {
        fn __deepcopy__(&self, _memo: &::pyo3::Bound<'_, ::pyo3::PyAny>) -> ::pyo3::PyResult<Self> {
            Ok(self.clone())
        }
    }
}

/// Generate `payload()` getter - returns rendered payload string
fn generate_payload() -> TokenStream {
    quote! {
        #[getter]
        pub fn payload(&self) -> ::pyo3::PyResult<::std::string::String> {
            <Self as ::trace_parser::common::TemplateEvent>::render_payload(self)
        }
    }
}

/// Generate `template()` getter - returns the template string
fn generate_template() -> TokenStream {
    quote! {
        #[getter]
        pub fn template(&self) -> &str {
            <Self as ::trace_parser::common::TemplateEvent>::formats().template(self.format_id).unwrap().template_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::FieldAttr;
    use syn::parse_quote;

    #[test]
    fn test_generate_new() {
        let code = generate_new(&[
            (
                parse_quote!(name),
                parse_quote!(String),
                FieldAttr {
                    name: None,
                    choice: vec![],
                    regex: None,
                    format: None,
                    readonly: false,
                    private: false,
                },
            ),
            (
                parse_quote!(value),
                parse_quote!(u32),
                FieldAttr {
                    name: None,
                    choice: vec![],
                    regex: None,
                    format: None,
                    readonly: false,
                    private: false,
                },
            ),
        ]);
        let code_str = code.to_string();
        assert!(code_str.contains("# [new]"));
        assert!(code_str.contains("fn new"));
        assert!(code_str.contains("name"));
        assert!(code_str.contains("value"));
    }

    #[test]
    fn test_generate_can_be_parsed() {
        let code = generate_can_be_parsed();
        let code_str = code.to_string();
        assert!(code_str.contains("# [staticmethod]"));
        assert!(code_str.contains("fn can_be_parsed"));
        assert!(code_str.contains("quick_check"));
    }

    #[test]
    fn test_generate_payload() {
        let code = generate_payload();
        let code_str = code.to_string();
        assert!(code_str.contains("# [getter]"));
        assert!(code_str.contains("fn payload"));
        assert!(code_str.contains("render_payload"));
    }

    #[test]
    fn test_generate_template() {
        let code = generate_template();
        let code_str = code.to_string();
        assert!(code_str.contains("# [getter]"));
        assert!(code_str.contains("fn template"));
        assert!(code_str.contains("template_str"));
    }

    #[test]
    fn test_generate_field_accessors_skips_private() {
        let accessors = generate_field_accessors(&[(
            parse_quote!(format_id),
            parse_quote!(u8),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
                readonly: false,
                private: true,
            },
        )]);
        assert!(accessors.is_empty());
    }

    #[test]
    fn test_generate_field_accessors_readonly_getter_only() {
        let accessors = generate_field_accessors(&[(
            parse_quote!(event_name),
            parse_quote!(String),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
                readonly: true,
                private: false,
            },
        )]);
        let code = accessors
            .iter()
            .map(TokenStream::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(code.contains("# [getter]"));
        assert!(!code.contains("# [setter]"));
    }

    #[test]
    fn test_generate_field_accessors_default_getter_and_setter() {
        let accessors = generate_field_accessors(&[(
            parse_quote!(thread_tid),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
                readonly: false,
                private: false,
            },
        )]);
        let code = accessors
            .iter()
            .map(TokenStream::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(code.contains("# [getter]"));
        assert!(code.contains("# [setter]"));
        assert!(code.contains("set_thread_tid"));
    }
}
