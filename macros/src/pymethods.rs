//! Python API generation for trace_event macros.

use crate::attrs::FieldAttr;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generate the complete `#[pymethods]` block for the struct
pub fn generate_pymethods_block(
    struct_name: &Ident,
    fields: &[(Ident, FieldAttr)],
) -> TokenStream {
    let new_fn = generate_new(fields);
    let repr_fn = generate_repr();
    let eq_fn = generate_eq(struct_name, fields);
    let str_fn = generate_str();
    let can_be_parsed_fn = generate_can_be_parsed();
    let parse_fn = generate_parse();
    let to_string_fn = generate_to_string();
    let copy_fn = generate_copy();
    let deepcopy_fn = generate_deepcopy();
    let payload_fn = generate_payload();
    let template_fn = generate_template();
    
    // Generate field getters/setters for Python API
    // Filters out private fields (not exposed to Python)
    let field_accessors = fields.iter().filter(|(_, attr)| !attr.private).map(|(field_name, field_attr)| {
        let ty = match field_attr.ty.as_str() {
            "string" => quote! { ::std::string::String },
            "u32" => quote! { u32 },
            "i32" => quote! { i32 },
            "f64" => quote! { f64 },
            "bool_int" => quote! { bool },
            _ => quote! { ::std::string::String },
        };

        if field_attr.optional {
            // Optional field: Python gets Option<T>, can be None
            if field_attr.readonly {
                // Optional + readonly: getter only, no setter
                quote! {
                    #[getter]
                    fn #field_name(&self) -> ::std::option::Option<#ty> {
                        self.#field_name
                    }
                }
            } else {
                // Optional + writable: getter and setter
                quote! {
                    #[getter]
                    fn #field_name(&self) -> ::std::option::Option<#ty> {
                        self.#field_name
                    }

                    #[setter]
                    fn #field_name(&mut self, value: ::std::option::Option<#ty>) {
                        self.#field_name = value;
                    }
                }
            }
        } else {
            // Required field (not optional)
            if field_attr.readonly {
                // Readonly: getter only
                quote! {
                    #[getter]
                    fn #field_name(&self) -> &#ty {
                        &self.#field_name
                    }
                }
            } else {
                // Writable: getter and setter
                quote! {
                    #[getter]
                    fn #field_name(&self) -> &#ty {
                        &self.#field_name
                    }
                    
                    #[setter]
                    fn #field_name(&mut self, value: #ty) {
                        self.#field_name = value;
                    }
                }
            }
        }
    });

    quote! {
        #[pyo3::pymethods]
        impl #struct_name {
            #new_fn
            #repr_fn
            #eq_fn
            #str_fn
            #can_be_parsed_fn
            #parse_fn
            #to_string_fn
            #copy_fn
            #deepcopy_fn
            #payload_fn
            #template_fn
            
            // Field getters/setters
            #(#field_accessors)*
        }
    }
}

/// Generate `#[new]` constructor
fn generate_new(fields: &[(Ident, FieldAttr)]) -> TokenStream {
    let field_names: Vec<&Ident> = fields.iter().map(|(name, _)| name).collect();
    
    let field_params: Vec<TokenStream> = fields.iter().map(|(field_name, field_attr)| {
        let ty = match field_attr.ty.as_str() {
            "string" => quote! { ::std::string::String },
            "u32" => quote! { u32 },
            "i32" => quote! { i32 },
            "f64" => quote! { f64 },
            "bool_int" => quote! { bool },
            _ => quote! { ::std::string::String },
        };
        
        if field_attr.optional {
            quote! { #field_name: ::std::option::Option<#ty> }
        } else {
            quote! { #field_name: #ty }
        }
    }).collect();

    let field_inits: Vec<TokenStream> = fields.iter().map(|(field_name, field_attr)| {
        if field_attr.optional {
            quote! { #field_name: #field_name.unwrap_or_default() }
        } else {
            quote! { #field_name }
        }
    }).collect();

    quote! {
        #[new]
        #[pyo3(signature = (#(#field_names),*))]
        #[allow(clippy::too_many_arguments)]
        fn new(#(#field_params),*) -> ::pyo3::PyResult<Self> {
            Ok(Self {
                #(#field_inits),*,
                // Base fields will be added separately
                thread_name: String::new(),
                thread_tid: 0,
                thread_tgid: 0,
                cpu: 0,
                flags: String::new(),
                timestamp: 0.0,
                event_name: Self::EVENT_NAME.to_string(),
                payload_raw: String::new(),
                format_id: 0,
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
fn generate_eq(struct_name: &Ident, fields: &[(Ident, FieldAttr)]) -> TokenStream {
    let field_comparisons: Vec<TokenStream> = fields.iter().map(|(field_name, _)| {
        quote! { self.#field_name == other.#field_name }
    }).collect();

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
        fn can_be_parsed(line: &str) -> bool {
            Self::quick_check(line)
        }
    }
}

/// Generate `parse` static method
fn generate_parse() -> TokenStream {
    quote! {
        #[staticmethod]
        fn parse(line: &str) -> ::std::option::Option<Self> {
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
        fn to_string(&self) -> ::pyo3::PyResult<::std::string::String> {
            ::trace_parser::trace::validate_timestamp(self.timestamp)?;
            let payload = self.payload()?;
            Ok(::trace_parser::trace::format_trace_header(
                &self.thread_name, self.thread_tid, self.thread_tgid, self.cpu,
                &self.flags, self.timestamp, &self.event_name,
                &payload
            ))
        }
    }
}

/// Generate `__copy__` method
fn generate_copy() -> TokenStream {
    quote! {
        fn __copy__(slf: ::pyo3::PyRef<'_, Self>, py: ::pyo3::Python<'_>) -> ::pyo3::PyResult<::pyo3::Py<Self>> {
            Ok(slf.clone().into_pyobject(py)?.unbind())
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
        fn payload(&self) -> ::pyo3::PyResult<::std::string::String> {
            self.render_payload()
        }
    }
}

/// Generate `template()` getter - returns the template string
fn generate_template() -> TokenStream {
    quote! {
        #[getter]
        fn template(&self) -> &str {
            Self::formats().template(self.format_id).unwrap().template_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn create_test_field(name: &str, ty: &str) -> (Ident, FieldAttr) {
        (
            syn::Ident::new(name, proc_macro2::Span::call_site()),
            FieldAttr {
                ty: ty.to_string(),
                name: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )
    }

    #[test]
    fn test_generate_new_basic() {
        let fields = vec![
            create_test_field("prev_comm", "string"),
            create_test_field("prev_pid", "u32"),
        ];

        let output = generate_new(&fields);
        // Just check it compiles and produces something with field names
        let output_str = output.to_string();

        assert!(output_str.contains("prev_comm"));
        assert!(output_str.contains("prev_pid"));
    }

    #[test]
    fn test_generate_new_with_optional() {
        let fields = vec![(
            parse_quote!(reason),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: true,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_new(&fields);

        assert!(output.to_string().contains("Option"));
    }

    #[test]
    fn test_generate_repr() {
        let output = generate_repr();

        assert!(output.to_string().contains("__repr__"));
    }

    #[test]
    fn test_generate_eq() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let fields = vec![create_test_field("prev_comm", "string")];

        let output = generate_eq(&struct_name, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("__eq__"));
        assert!(output_str.contains("prev_comm"));
    }

    #[test]
    fn test_generate_can_be_parsed() {
        let output = generate_can_be_parsed();

        assert!(output.to_string().contains("can_be_parsed"));
        assert!(output.to_string().contains("quick_check"));
    }

    #[test]
    fn test_generate_payload() {
        let output = generate_payload();
        let output_str = output.to_string();

        assert!(output_str.contains("payload"));
        assert!(output_str.contains("render_payload"));
    }

    #[test]
    fn test_generate_template() {
        let output = generate_template();
        let output_str = output.to_string();

        assert!(output_str.contains("template"));
        assert!(output_str.contains("formats"));
    }

    #[test]
    fn test_generate_pymethods_block_with_field_accessors() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let fields = vec![
            create_test_field("value", "u32"),
            create_test_field("name", "string"),
        ];

        let output = generate_pymethods_block(&struct_name, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("getter"));
        assert!(output_str.contains("value"));
        assert!(output_str.contains("name"));
    }

    #[test]
    fn test_generate_pymethods_block_with_optional_field() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let fields = vec![(
            parse_quote!(optional_value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: true,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_pymethods_block(&struct_name, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("Option"));
        assert!(output_str.contains("optional_value"));
    }

    #[test]
    fn test_generate_pymethods_block_excludes_private_field() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let fields = vec![
            create_test_field("public", "u32"),
            (
                parse_quote!(private_field),
                FieldAttr {
                    ty: "u32".to_string(),
                    name: None,
                    optional: false,
                    readonly: false,
                    private: true,
                },
            ),
        ];

        let output = generate_pymethods_block(&struct_name, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("public"));
        // private_field не должен быть в геттерах, но может быть в конструкторе
        // Проверяем что нет getter для private_field
        assert!(!output_str.contains("fn private_field"));
    }
}
