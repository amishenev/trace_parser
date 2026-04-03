//! Code generation for trace_event macros.

use crate::attrs::{DefineTemplateAttr, FieldAttr, TraceEventAttr, TraceMarkersAttr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

/// Базовые поля трассировки, которые НЕ включаются в payload template
/// но автоматически добавляются в parse_payload из BaseTraceParts
const BASE_FIELDS: &[&str] = &[
    "thread_name",
    "thread_tid",
    "thread_tgid",
    "cpu",
    "flags",
    "timestamp",
    "event_name",
    "format_id",
];

/// Проверить является ли поле базовым
fn is_base_field(name: &str) -> bool {
    BASE_FIELDS.contains(&name)
}

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

    // Compute auto-assigned ids for templates
    // Logic: id = max(previous_ids) + 1 for None values
    let mut max_seen_id: Option<u8> = None;
    let template_ids: Vec<u8> = templates
        .iter()
        .map(|t| {
            let result = t.id.unwrap_or_else(|| max_seen_id.map_or(0u8, |m| m + 1));
            // Update max with the result (whether explicit or auto-assigned)
            if max_seen_id.is_none_or(|m| result > m) {
                max_seen_id = Some(result);
            }
            result
        })
        .collect();

    // Generate field specs for the template (used in parse_payload and template construction)
    // Исключаем базовые поля — они не входят в payload template
    let field_specs: Vec<TokenStream> = fields
        .iter()
        .filter(|(field_name, _)| !is_base_field(&field_name.to_string()))
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

    // Generate format registry with all templates
    // Each template needs a static LazyLock to avoid temporary value issues
    let template_statics: Vec<TokenStream> = templates
        .iter()
        .zip(template_ids.iter())
        .map(|(template_attr, id)| {
            let template_str = &template_attr.template;
            let template_name = syn::Ident::new(
                &format!("TEMPLATE_{}", id),
                proc_macro2::Span::call_site(),
            );

            quote! {
                static #template_name: ::std::sync::LazyLock<::trace_parser::payload_template::PayloadTemplate> =
                    ::std::sync::LazyLock::new(|| {
                        ::trace_parser::payload_template::PayloadTemplate::new(
                            #template_str,
                            &[#(#field_specs),*]
                        )
                    });
            }
        })
        .collect();

    let format_specs: Vec<TokenStream> = templates
        .iter()
        .zip(template_ids.iter())
        .map(|(_template_attr, id)| {
            let template_name = syn::Ident::new(
                &format!("TEMPLATE_{}", id),
                proc_macro2::Span::call_site(),
            );

            quote! {
                ::trace_parser::format_registry::FormatSpec {
                    kind: #id,
                    template: &#template_name,
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

    // Generate render statements for render_payload
    // Исключаем базовые поля — они не рендерятся в payload
    let render_statements: Vec<TokenStream> = fields
        .iter()
        .filter(|(field_name, _)| !is_base_field(&field_name.to_string()))
        .map(|(field_name, field_attr)| {
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());
            let ty = &field_attr.ty;

            if field_attr.optional {
                // Optional field: wrap in Some/None
                match ty.as_str() {
                    "string" => quote! {
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(v)))
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
                        (#name_str, self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(v)))
                    },
                }
            } else {
                // Required field: always Some
                match ty.as_str() {
                    "string" => quote! {
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::Str(&self.#field_name)))
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
                        (#name_str, Some(::trace_parser::payload_template::TemplateValue::Str(&self.#field_name)))
                    },
                }
            }
        })
        .collect();

    // Generate parse statements for parse_payload
    // Исключаем базовые поля — они добавляются вручную из parts
    let parse_statements: Vec<TokenStream> = fields
        .iter()
        .filter(|(field_name, _)| !is_base_field(&field_name.to_string()))
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
        #(#template_statics)*

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
                    format_id: _format_id,

                    // Payload поля
                    #(#parse_statements),*
                })
            }

            fn render_payload(&self) -> ::pyo3::PyResult<::std::string::String> {
                let template = Self::formats().template(0)
                    .ok_or_else(|| ::pyo3::exceptions::PyRuntimeError::new_err("No template found"))?;

                let values = &[
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
    use rstest::rstest;
    use syn::parse_quote;

    /// Integration test: verify auto-assign id logic in generated code
    #[rstest]
    #[case(vec![None, None], vec![0, 1])]
    #[case(vec![Some(0), None], vec![0, 1])]
    #[case(vec![Some(1), Some(2), None], vec![1, 2, 3])]
    #[case(vec![None, Some(1), None], vec![0, 1, 2])]
    #[case(vec![Some(0), None, Some(5), None], vec![0, 1, 5, 6])]
    #[case(vec![Some(0), None, Some(5)], vec![0, 1, 5])]
    fn test_template_id_auto_assign(
        #[case] ids: Vec<Option<u8>>,
        #[case] expected: Vec<u8>,
    ) {
        let struct_name: Ident = parse_quote!(TestEvent);
        
        // Build templates from input ids
        let templates: Vec<DefineTemplateAttr> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| DefineTemplateAttr {
                template: format!("field{}={{field{}}}", i, i),
                id: *id,
            })
            .collect();
        
        // Build fields
        let fields: Vec<(Ident, FieldAttr)> = (0..ids.len())
            .map(|i| {
                (
                    syn::Ident::new(&format!("field{}", i), proc_macro2::Span::call_site()),
                    FieldAttr {
                        ty: "u32".to_string(),
                        name: None,
                        optional: false,
                        readonly: false,
                        private: false,
                    },
                )
            })
            .collect();

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Check all expected ids are present
        for id in expected {
            let pattern = format!("kind : {}u8", id);
            assert!(
                output_str.contains(&pattern),
                "Expected '{}' in generated code:\n{}",
                pattern,
                output_str
            );
        }
    }

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
}
