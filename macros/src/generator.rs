//! Code generation for trace_event macros.

use crate::attrs::{DefineTemplateAttr, FieldAttr, TraceEventAttr, TraceMarkersAttr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Generate `impl EventType` for the struct
pub fn generate_event_type_impl(
    struct_name: &Ident,
    event_attr: &TraceEventAttr,
) -> TokenStream {
    let name = &event_attr.name;
    let aliases = &event_attr.aliases;

    quote! {
        impl ::trace_parser::common::EventType for #struct_name {
            const EVENT_NAME: &'static str = #name;
            const EVENT_ALIASES: &'static [&'static str] = &[#(#aliases),*];
        }
    }
}

/// Generate `impl FastMatch` for the struct
pub fn generate_fast_match_impl(
    struct_name: &Ident,
    markers_attr: Option<&TraceMarkersAttr>,
) -> TokenStream {
    if let Some(attr) = markers_attr {
        let markers = &attr.0;
        // Convert string markers to byte slices
        let marker_bytes: Vec<TokenStream> = markers
            .iter()
            .map(|m| {
                let bytes = m.as_bytes();
                quote! { &[#(#bytes),*] }
            })
            .collect();

        quote! {
            impl ::trace_parser::common::FastMatch for #struct_name {
                const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[#(#marker_bytes),*];
            }
        }
    } else {
        // No markers - use default empty implementation
        quote! {
            impl ::trace_parser::common::FastMatch for #struct_name {}
        }
    }
}

/// Generate `impl TemplateEvent` for the struct
pub fn generate_template_event_impl(
    struct_name: &Ident,
    templates: &[DefineTemplateAttr],
    fields: &[(Ident, FieldAttr)],
) -> TokenStream {
    if templates.is_empty() {
        return quote! {};
    }

    // Generate format registry with all templates
    // Auto-assign ids: templates without explicit id get sequential ids starting after max explicit id
    // Example: explicit ids [1, 2] → auto-assign starts from 3
    let max_explicit_id = templates
        .iter()
        .filter_map(|t| t.id)
        .max()
        .unwrap_or(255);
    
    let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
    
    let format_specs: Vec<TokenStream> = templates
        .iter()
        .map(|template_attr| {
            let template_str = &template_attr.template;
            let id = template_attr.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            });
            
            quote! {
                ::trace_parser::format_registry::FormatSpec {
                    kind: #id,
                    template: &::trace_parser::payload_template::PayloadTemplate::new(
                        #template_str,
                        &[]
                    )
                }
            }
        })
        .collect();

    // Generate detect_format logic
    // For single template: always return 0
    // For multiple templates: check for unique fields
    let detect_format_impl = if templates.len() == 1 {
        quote! {
            fn detect_format(_payload: &str) -> u8 {
                0
            }
        }
    } else {
        // For multiple templates, detect by checking for unique fields
        // Simple heuristic: check templates in reverse order (most specific first)
        let checks: Vec<TokenStream> = templates
            .iter()
            .rev()
            .filter_map(|template_attr| {
                let id = template_attr.id.unwrap_or(0);
                if id == 0 {
                    // Skip format 0 (default fallback)
                    None
                } else {
                    // Extract field names from template to check presence
                    // Simple check: look for "field=" pattern
                    let template_str = &template_attr.template;
                    Some(quote! {
                        if payload.contains(#template_str) {
                            return #id;
                        }
                    })
                }
            })
            .collect();
        
        if checks.is_empty() {
            quote! {
                fn detect_format(_payload: &str) -> u8 {
                    0
                }
            }
        } else {
            quote! {
                fn detect_format(payload: &str) -> u8 {
                    #(#checks)*
                    0
                }
            }
        }
    };

    // Generate field specs for the template (used in parse_payload)
    let _field_specs: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, field_attr)| {
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());
            let ty = &field_attr.ty;

            let field_spec = match ty.as_str() {
                "string" => quote! { ::trace_parser::payload_template::FieldSpec::string() },
                "u32" => quote! { ::trace_parser::payload_template::FieldSpec::u32() },
                "i32" => quote! { ::trace_parser::payload_template::FieldSpec::i32() },
                "f64" => quote! { ::trace_parser::payload_template::FieldSpec::f64() },
                "bool_int" => quote! { ::trace_parser::payload_template::FieldSpec::bool_int() },
                _ => quote! { ::trace_parser::payload_template::FieldSpec::custom(r".+") },
            };

            quote! {
                (#name_str, #field_spec)
            }
        })
        .collect();

    // Generate render statements for render_payload
    let render_statements: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, field_attr)| {
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());
            let ty = &field_attr.ty;

            if field_attr.optional {
                // Optional field: wrap in Some/None
                match ty.as_str() {
                    "string" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(v.as_str())))
                    },
                    "u32" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::U32(*v)))
                    },
                    "i32" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::I32(*v)))
                    },
                    "f64" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::F64(*v)))
                    },
                    "bool_int" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::BoolInt(*v)))
                    },
                    _ => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(v.as_str())))
                    },
                }
            } else {
                // Required field: always Some
                match ty.as_str() {
                    "string" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::Str(self.#field_name.as_str())))
                    },
                    "u32" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::U32(self.#field_name)))
                    },
                    "i32" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::I32(self.#field_name)))
                    },
                    "f64" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::F64(self.#field_name)))
                    },
                    "bool_int" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::BoolInt(self.#field_name)))
                    },
                    _ => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::Str(self.#field_name.as_str())))
                    },
                }
            }
        })
        .collect();

    // Generate parse statements for parse_payload
    let parse_statements: Vec<TokenStream> = fields
        .iter()
        .map(|(field_name, field_attr)| {
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());
            let ty = &field_attr.ty;

            if field_attr.optional {
                // Optional field: returns Option<T>
                match ty.as_str() {
                    "string" => quote! {
                        #field_name: ::trace_parser::common::cap_str(captures, #name_str)
                    },
                    "u32" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<u32>(captures, #name_str)
                    },
                    "i32" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<i32>(captures, #name_str)
                    },
                    "f64" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<f64>(captures, #name_str)
                    },
                    "bool_int" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<u8>(captures, #name_str).map(|v| v == 1)
                    },
                    _ => quote! {
                        #field_name: ::trace_parser::common::cap_str(captures, #name_str)
                    },
                }
            } else {
                // Required field: uses ? operator
                match ty.as_str() {
                    "string" => quote! {
                        #field_name: ::trace_parser::common::cap_str(captures, #name_str)?
                    },
                    "u32" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<u32>(captures, #name_str)?
                    },
                    "i32" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<i32>(captures, #name_str)?
                    },
                    "f64" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<f64>(captures, #name_str)?
                    },
                    "bool_int" => quote! {
                        #field_name: ::trace_parser::common::cap_parse::<u8>(captures, #name_str)? == 1
                    },
                    _ => quote! {
                        #field_name: ::trace_parser::common::cap_str(captures, #name_str)?
                    },
                }
            }
        })
        .collect();

    quote! {
        impl ::trace_parser::common::TemplateEvent for #struct_name {
            fn formats() -> &'static ::trace_parser::format_registry::FormatRegistry {
                static FORMATS: ::std::sync::LazyLock<::trace_parser::format_registry::FormatRegistry> =
                    ::std::sync::LazyLock::new(|| {
                        ::trace_parser::format_registry::FormatRegistry::new(vec![
                            #(#format_specs),*
                        ])
                    });
                &FORMATS
            }

            #detect_format_impl

            fn parse_payload(
                parts: ::trace_parser::common::BaseTraceParts,
                captures: &::regex::Captures<'_>,
                _format_id: u8,
            ) -> ::std::option::Option<Self> {
                Some(Self {
                    // Базовые поля
                    thread_name: parts.thread_name,
                    thread_tid: parts.thread_tid,
                    thread_tgid: parts.thread_tgid,
                    cpu: parts.cpu,
                    flags: parts.flags,
                    timestamp: parts.timestamp,
                    event_name: parts.event_name,
                    payload_raw: parts.payload_raw,
                    format_id: _format_id,
                    
                    // Payload поля
                    #(#parse_statements),*
                })
            }

            fn render_payload(&self) -> ::pyo3::PyResult<::std::string::String> {
                let template = Self::formats().template(0)
                    .ok_or_else(|| ::pyo3::exceptions::PyRuntimeError::new_err("No template found"))?;
                
                let values: &[(&str, ::std::option::Option<::trace_parser::payload_template::TemplateValue>)] = &[
                    #(#render_statements),*
                ];
                
                template.format(values)
                    .ok_or_else(|| ::pyo3::exceptions::PyRuntimeError::new_err("Failed to format template"))
            }
        }
    }
}

/// Generate registration code for regular trace events
pub fn generate_registration(struct_name: &Ident, event_attr: &TraceEventAttr) -> TokenStream {
    let name = &event_attr.name;
    
    quote! {
        ::trace_parser::register_parser!(#name, #struct_name);
    }
}

/// Generate registration code for tracing_mark events
pub fn generate_tracing_mark_registration(struct_name: &Ident) -> TokenStream {
    quote! {
        ::trace_parser::register_tracing_mark_parser!(#struct_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn test_generate_event_type_impl() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let event_attr = TraceEventAttr {
            name: "sched_switch".to_string(),
            aliases: vec!["sched_sw".to_string()],
        };

        let output = generate_event_type_impl(&struct_name, &event_attr);

        let expected = quote! {
            impl ::trace_parser::common::EventType for TraceSchedSwitch {
                const EVENT_NAME: &'static str = "sched_switch";
                const EVENT_ALIASES: &'static [&'static str] = &["sched_sw"];
            }
        };

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_fast_match_impl_with_markers() {
        let struct_name: Ident = parse_quote!(TraceMarkBegin);
        let markers_attr = Some(TraceMarkersAttr(vec!["B|".to_string()]));

        let output = generate_fast_match_impl(&struct_name, markers_attr.as_ref());
        let output_str = output.to_string();

        assert!(output_str.contains("FastMatch"));
        assert!(output_str.contains("B"));
    }

    #[test]
    fn test_generate_fast_match_impl_empty() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let markers_attr: Option<TraceMarkersAttr> = None;

        let output = generate_fast_match_impl(&struct_name, markers_attr.as_ref());
        let output_str = output.to_string();

        assert!(output_str.contains("FastMatch"));
    }

    #[test]
    fn test_generate_registration() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let event_attr = TraceEventAttr {
            name: "sched_switch".to_string(),
            aliases: vec![],
        };

        let output = generate_registration(&struct_name, &event_attr);
        let output_str = output.to_string();

        assert!(output_str.contains("register_parser"));
        assert!(output_str.contains("TraceSchedSwitch"));
    }

    #[test]
    fn test_generate_tracing_mark_registration() {
        let struct_name: Ident = parse_quote!(TraceMarkBegin);

        let output = generate_tracing_mark_registration(&struct_name);
        let output_str = output.to_string();

        assert!(output_str.contains("register_tracing_mark_parser"));
        assert!(output_str.contains("TraceMarkBegin"));
    }

    #[test]
    fn test_generate_template_event_impl_with_render() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("render_payload"));
        assert!(output_str.contains("TemplateValue :: U32"));
        assert!(output_str.contains("value"));
    }

    #[test]
    fn test_generate_template_event_impl_with_optional() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: true,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("render_payload"));
        assert!(output_str.contains("as_ref () . map"));
    }

    #[test]
    fn test_generate_template_event_impl_with_custom_name() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "state={state}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(current_state),
            FieldAttr {
                ty: "u32".to_string(),
                name: Some("state".to_string()),
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("\"state\""));
        assert!(output_str.contains("current_state"));
    }

    #[test]
    fn test_generate_template_event_impl_with_parse() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("parse_payload"));
        assert!(output_str.contains("cap_parse :: < u32 >"));
        assert!(output_str.contains("Some (Self"));
    }

    #[test]
    fn test_generate_template_event_impl_with_parse_optional() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: true,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("parse_payload"));
        assert!(output_str.contains("cap_parse :: < u32 >"));
        // Optional field doesn't use ? operator
        assert!(!output_str.contains("cap_parse :: < u32 > (captures , \"value\") ?"));
    }

    #[test]
    fn test_generate_template_event_impl_with_parse_bool_int() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "flag={flag}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(flag),
            FieldAttr {
                ty: "bool_int".to_string(),
                name: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("parse_payload"));
        assert!(output_str.contains("cap_parse :: < u8 >"));
        assert!(output_str.contains("== 1"));
    }

    #[test]
    fn test_generate_template_event_impl_with_multiple_templates() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(1) },
        ];
        let fields = vec![
            (parse_quote!(a), FieldAttr { ty: "u32".to_string(), name: None, optional: false, readonly: false, private: false }),
            (parse_quote!(b), FieldAttr { ty: "u32".to_string(), name: None, optional: true, readonly: false, private: false }),
        ];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("detect_format"));
        assert!(output_str.contains("FormatSpec"));
    }

    #[test]
    fn test_generate_template_event_impl_single_template_detect_format() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None }];
        let fields = vec![(
            parse_quote!(value),
            FieldAttr {
                ty: "u32".to_string(),
                name: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Single template: detect_format always returns 0
        assert!(output_str.contains("fn detect_format (_payload : & str) -> u8 { 0 }"));
    }

    #[test]
    fn test_template_id_fully_explicit() {
        // Both templates with explicit ids
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(1) },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn test_template_id_fully_implicit() {
        // Both templates without explicit ids (auto-assign from 0)
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: None },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: None },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn test_template_id_mixed_explicit_first() {
        // First template with explicit id=0, second without (auto-assign starts after max=0, so gets 1)
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: None },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![0, 1]); // Second gets 1 (after max explicit 0)
    }

    #[test]
    fn test_template_id_mixed_explicit_second() {
        // First template without explicit id (gets 0), second with explicit 1
        // max_explicit=1, so auto starts from 2
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: None },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(1) },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![2, 1]); // First gets 2 (after max explicit 1), second explicit 1
    }

    #[test]
    fn test_template_id_three_templates_mixed() {
        // Three templates with mixed explicit/implicit ids
        // Explicit: 0, 2. Max=2, so auto starts from 3
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: None },
            DefineTemplateAttr { template: "a={a} b={b} c={c}".to_string(), id: Some(2) },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![0, 3, 2]); // Second gets 3 (after max explicit 2)
    }

    #[test]
    fn test_template_id_explicit_middle() {
        // First and third implicit, second explicit=1
        // max_explicit=1, so auto starts from 2
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: None },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(1) },
            DefineTemplateAttr { template: "a={a} b={b} c={c}".to_string(), id: None },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![2, 1, 3]); // First gets 2, third gets 3
    }

    #[test]
    fn test_template_id_auto_assign_skips_explicit() {
        // First two explicit (1, 2), third implicit
        // max_explicit=2, so auto starts from 3
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(1) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(2) },
            DefineTemplateAttr { template: "a={a} b={b} c={c}".to_string(), id: None },
        ];
        let fields = vec![
            (parse_quote!(a), FieldAttr { ty: "u32".to_string(), name: None, optional: false, readonly: false, private: false }),
            (parse_quote!(b), FieldAttr { ty: "u32".to_string(), name: None, optional: true, readonly: false, private: false }),
            (parse_quote!(c), FieldAttr { ty: "u32".to_string(), name: None, optional: true, readonly: false, private: false }),
        ];
        
        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();
        
        println!("Generated:\n{}", output_str);
        
        // Check that format ids 1, 2, 3 are present
        assert!(output_str.contains("kind : 1u8"));
        assert!(output_str.contains("kind : 2u8"));
        assert!(output_str.contains("kind : 3u8"));
    }

    #[test]
    fn test_template_id_auto_assign_with_gap() {
        // First explicit=0, second implicit, third explicit=5
        // max_explicit=5, so auto starts from 6
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: None },
            DefineTemplateAttr { template: "a={a} b={b} c={c}".to_string(), id: Some(5) },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![0, 6, 5]); // Second gets 6 (after max explicit 5)
    }

    #[test]
    fn test_template_id_user_scenario_explicit_1_2_auto_3() {
        // User scenario: first id=1, second id=2, third None
        // Expected: third gets id=3 (continues after max explicit)
        let templates = vec![
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(1) },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(2) },
            DefineTemplateAttr { template: "a={a} b={b} c={c}".to_string(), id: None },
        ];
        
        let max_explicit_id = templates.iter().filter_map(|t| t.id).max().unwrap_or(255);
        let mut next_auto_id = if max_explicit_id == 255 { 0u8 } else { max_explicit_id + 1 };
        let ids: Vec<u8> = templates
            .iter()
            .map(|t| t.id.unwrap_or_else(|| {
                let assigned = next_auto_id;
                next_auto_id += 1;
                assigned
            }))
            .collect();
        
        assert_eq!(ids, vec![1, 2, 3]); // Third gets 3!
    }
}
