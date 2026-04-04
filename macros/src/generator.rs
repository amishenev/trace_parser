//! Code generation for trace_event macros.

use crate::attrs::{DefineTemplateAttr, FieldAttr, TraceEventAttr, TraceMarkersAttr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

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

/// Тип поля для генерации кода парсинга/рендера
pub(crate) enum InferredType {
    String,
    U8, U16, U32, U64,
    I8, I16, I32, I64,
    F32, F64,
    Bool,
}

impl InferredType {
    /// Вывод типа из Rust-типа поля
    pub(crate) fn from_syn(ty: &Type) -> Option<Self> {
        match ty {
            Type::Path(tp) => {
                let seg = tp.path.segments.last()?;
                match seg.ident.to_string().as_str() {
                    "String" => Some(Self::String),
                    "u8" => Some(Self::U8),
                    "u16" => Some(Self::U16),
                    "u32" => Some(Self::U32),
                    "u64" => Some(Self::U64),
                    "i8" => Some(Self::I8),
                    "i16" => Some(Self::I16),
                    "i32" => Some(Self::I32),
                    "i64" => Some(Self::I64),
                    "f32" => Some(Self::F32),
                    "f64" => Some(Self::F64),
                    "bool" => Some(Self::Bool),
                    "Option" => {
                        // Option<T> — извлекаем T
                        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
                        {
                            return Self::from_syn(inner);
                        }
                        None
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn field_spec(&self, custom_regex: Option<&str>, choice_values: Option<&[String]>) -> TokenStream {
        if let Some(values) = choice_values
            && !values.is_empty()
        {
            return quote! { ::trace_parser::payload_template::FieldSpec::choice(&[#(#values),*]) };
        }
        if let Some(regex) = custom_regex {
            return quote! { ::trace_parser::payload_template::FieldSpec::custom(#regex) };
        }
        match self {
            Self::String => quote! { ::trace_parser::payload_template::FieldSpec::string() },
            Self::U8 => quote! { ::trace_parser::payload_template::FieldSpec::u32() },
            Self::U16 => quote! { ::trace_parser::payload_template::FieldSpec::u32() },
            Self::U32 => quote! { ::trace_parser::payload_template::FieldSpec::u32() },
            Self::U64 => quote! { ::trace_parser::payload_template::FieldSpec::custom(r"\d+") },
            Self::I8 => quote! { ::trace_parser::payload_template::FieldSpec::i32() },
            Self::I16 => quote! { ::trace_parser::payload_template::FieldSpec::i32() },
            Self::I32 => quote! { ::trace_parser::payload_template::FieldSpec::i32() },
            Self::I64 => quote! { ::trace_parser::payload_template::FieldSpec::custom(r"-?\d+") },
            Self::F32 => quote! { ::trace_parser::payload_template::FieldSpec::f64() },
            Self::F64 => quote! { ::trace_parser::payload_template::FieldSpec::f64() },
            Self::Bool => quote! { ::trace_parser::payload_template::FieldSpec::bool_int() },
        }
    }

    fn parse_code(&self, captures: &TokenStream, name: &str, custom_regex: Option<&str>) -> TokenStream {
        let name = name.to_string();
        // Custom regex: always parse as string, then convert
        if custom_regex.is_some() {
            return match self {
                Self::String => quote! { ::trace_parser::common::cap_str(#captures, #name)? },
                Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::I8 | Self::I16 | Self::I32 | Self::I64 => {
                    quote! { ::trace_parser::common::cap_str(#captures, #name)?.parse().ok()? }
                }
                Self::F32 | Self::F64 => {
                    quote! { ::trace_parser::common::cap_str(#captures, #name)?.parse().ok()? }
                }
                Self::Bool => quote! { ::trace_parser::common::cap_str(#captures, #name)? == "1" },
            };
        }
        // Default: use type-specific parsing
        match self {
            Self::String => quote! { ::trace_parser::common::cap_str(#captures, #name)? },
            Self::U8 => quote! { ::trace_parser::common::cap_parse::<u8>(#captures, #name)? },
            Self::U16 => quote! { ::trace_parser::common::cap_parse::<u16>(#captures, #name)? },
            Self::U32 => quote! { ::trace_parser::common::cap_parse::<u32>(#captures, #name)? },
            Self::U64 => quote! { ::trace_parser::common::cap_parse::<u64>(#captures, #name)? },
            Self::I8 => quote! { ::trace_parser::common::cap_parse::<i8>(#captures, #name)? },
            Self::I16 => quote! { ::trace_parser::common::cap_parse::<i16>(#captures, #name)? },
            Self::I32 => quote! { ::trace_parser::common::cap_parse::<i32>(#captures, #name)? },
            Self::I64 => quote! { ::trace_parser::common::cap_parse::<i64>(#captures, #name)? },
            Self::F32 => quote! { ::trace_parser::common::cap_parse::<f32>(#captures, #name)? },
            Self::F64 => quote! { ::trace_parser::common::cap_parse::<f64>(#captures, #name)? },
            Self::Bool => quote! { ::trace_parser::common::cap_parse::<u8>(#captures, #name)? == 1 },
        }
    }

    fn parse_optional_code(&self, captures: &TokenStream, name: &str, custom_regex: Option<&str>) -> TokenStream {
        let name = name.to_string();
        if custom_regex.is_some() {
            return match self {
                Self::String => quote! { ::trace_parser::common::cap_str(#captures, #name) },
                Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::I8 | Self::I16 | Self::I32 | Self::I64 => {
                    quote! { ::trace_parser::common::cap_str(#captures, #name).and_then(|s| s.parse().ok()) }
                }
                Self::F32 | Self::F64 => {
                    quote! { ::trace_parser::common::cap_str(#captures, #name).and_then(|s| s.parse().ok()) }
                }
                Self::Bool => quote! { ::trace_parser::common::cap_str(#captures, #name).map(|v| v == "1") },
            };
        }
        match self {
            Self::String => quote! { ::trace_parser::common::cap_str(#captures, #name) },
            Self::U8 => quote! { ::trace_parser::common::cap_parse::<u8>(#captures, #name) },
            Self::U16 => quote! { ::trace_parser::common::cap_parse::<u16>(#captures, #name) },
            Self::U32 => quote! { ::trace_parser::common::cap_parse::<u32>(#captures, #name) },
            Self::U64 => quote! { ::trace_parser::common::cap_parse::<u64>(#captures, #name) },
            Self::I8 => quote! { ::trace_parser::common::cap_parse::<i8>(#captures, #name) },
            Self::I16 => quote! { ::trace_parser::common::cap_parse::<i16>(#captures, #name) },
            Self::I32 => quote! { ::trace_parser::common::cap_parse::<i32>(#captures, #name) },
            Self::I64 => quote! { ::trace_parser::common::cap_parse::<i64>(#captures, #name) },
            Self::F32 => quote! { ::trace_parser::common::cap_parse::<f32>(#captures, #name) },
            Self::F64 => quote! { ::trace_parser::common::cap_parse::<f64>(#captures, #name) },
            Self::Bool => quote! { ::trace_parser::common::cap_parse::<u8>(#captures, #name).map(|v| v == 1) },
        }
    }

    fn render_value(&self, field_name: &Ident) -> TokenStream {
        match self {
            Self::String => quote! { Some(::trace_parser::payload_template::TemplateValue::Str(&self.#field_name)) },
            Self::U8 => quote! { Some(::trace_parser::payload_template::TemplateValue::U32(self.#field_name as u32)) },
            Self::U16 => quote! { Some(::trace_parser::payload_template::TemplateValue::U32(self.#field_name as u32)) },
            Self::U32 => quote! { Some(::trace_parser::payload_template::TemplateValue::U32(self.#field_name)) },
            Self::U64 => quote! { Some(::trace_parser::payload_template::TemplateValue::Str(&self.#field_name.to_string())) },
            Self::I8 => quote! { Some(::trace_parser::payload_template::TemplateValue::I32(self.#field_name as i32)) },
            Self::I16 => quote! { Some(::trace_parser::payload_template::TemplateValue::I32(self.#field_name as i32)) },
            Self::I32 => quote! { Some(::trace_parser::payload_template::TemplateValue::I32(self.#field_name)) },
            Self::I64 => quote! { Some(::trace_parser::payload_template::TemplateValue::Str(&self.#field_name.to_string())) },
            Self::F32 => quote! { Some(::trace_parser::payload_template::TemplateValue::F64(self.#field_name as f64)) },
            Self::F64 => quote! { Some(::trace_parser::payload_template::TemplateValue::F64(self.#field_name)) },
            Self::Bool => quote! { Some(::trace_parser::payload_template::TemplateValue::BoolInt(self.#field_name)) },
        }
    }

    fn render_optional_value(&self, field_name: &Ident) -> TokenStream {
        match self {
            Self::String => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(v)) },
            Self::U8 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::U32(*v as u32)) },
            Self::U16 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::U32(*v as u32)) },
            Self::U32 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::U32(*v)) },
            Self::U64 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(&v.to_string())) },
            Self::I8 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::I32(*v as i32)) },
            Self::I16 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::I32(*v as i32)) },
            Self::I32 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::I32(*v)) },
            Self::I64 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::Str(&v.to_string())) },
            Self::F32 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::F64(*v as f64)) },
            Self::F64 => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::F64(*v)) },
            Self::Bool => quote! { self.#field_name.as_ref().map(|v| ::trace_parser::payload_template::TemplateValue::BoolInt(*v)) },
        }
    }

    pub(crate) fn rust_type_tokens(&self) -> TokenStream {
        match self {
            Self::String => quote! { ::std::string::String },
            Self::U8 => quote! { u8 },
            Self::U16 => quote! { u16 },
            Self::U32 => quote! { u32 },
            Self::U64 => quote! { u64 },
            Self::I8 => quote! { i8 },
            Self::I16 => quote! { i16 },
            Self::I32 => quote! { i32 },
            Self::I64 => quote! { i64 },
            Self::F32 => quote! { f32 },
            Self::F64 => quote! { f64 },
            Self::Bool => quote! { bool },
        }
    }
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
    payload_contains_any: &[String],
) -> TokenStream {
    let markers = markers_attr.map(|a| a.0.as_slice()).unwrap_or(&[]);
    let marker_bytes: Vec<TokenStream> = markers
        .iter()
        .map(|m| {
            let bytes = m.as_bytes();
            quote! { &[#(#bytes),*] }
        })
        .collect();

    let quick_check = if payload_contains_any.is_empty() {
        quote! {}
    } else {
        quote! {
            fn payload_quick_check(line: &str) -> bool {
                ::trace_parser::common::contains_any(line, &[#(#payload_contains_any),*])
            }
        }
    };

    quote! {
        impl ::trace_parser::common::FastMatch for #struct_name {
            const PAYLOAD_MARKERS: &'static [&'static [u8]] = &[#(#marker_bytes),*];
            #quick_check
        }
    }
}

/// Generate `impl TemplateEvent` for the struct
pub fn generate_template_event_impl(
    struct_name: &Ident,
    templates: &[DefineTemplateAttr],
    fields: &[(Ident, Type, FieldAttr)],
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
        .filter(|(field_name, _, _)| !is_base_field(&field_name.to_string()))
        .map(|(field_name, field_ty, field_attr)| {
            let inferred = InferredType::from_syn(field_ty)
                .expect("unsupported field type for template");
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());

            let choice = if field_attr.choice.is_empty() { None } else { Some(field_attr.choice.as_slice()) };
            let field_spec = inferred.field_spec(field_attr.regex.as_deref(), choice);

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
                &format!("__TRACE_PARSER_{}_TMPL_{}", struct_name, id),
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
                &format!("__TRACE_PARSER_{}_TMPL_{}", struct_name, id),
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
    // If any template has detect markers → SIMD-based detection
    // Otherwise → call detect_format_override (inherent method, defaults to 0)
    let detect_markers: Vec<(Vec<u8>, u8)> = templates
        .iter()
        .zip(template_ids.iter())
        .filter(|(t, _)| !t.detect.is_empty())
        .flat_map(|(t, &id)| {
            t.detect.iter().map(move |d| (d.as_bytes().to_vec(), id))
        })
        .collect();

    let detect_format_impl = if !detect_markers.is_empty() {
        let markers: Vec<TokenStream> = detect_markers
            .iter()
            .map(|(bytes, id)| {
                let byte_values: Vec<_> = bytes.iter().map(|b| quote! { #b }).collect();
                quote! {
                    (&[#(#byte_values),*], #id)
                }
            })
            .collect();

        quote! {
            fn detect_format(payload: &str) -> u8 {
                const MARKERS: &'static [(&[u8], u8)] = &[#(#markers),*];
                for (marker, id) in MARKERS {
                    if memchr::memmem::find(payload.as_bytes(), *marker).is_some() {
                        return *id;
                    }
                }
                0
            }
        }
    } else {
        quote! {
            fn detect_format(payload: &str) -> u8 {
                Self::detect_format_override(payload)
            }
        }
    };

    // Generate detect_format_override inherent method (default: return 0)
    let detect_format_override = if detect_markers.is_empty() {
        quote! {
            impl #struct_name {
                /// Override this method for custom format detection logic.
                fn detect_format_override(_payload: &str) -> u8 {
                    0
                }
            }
        }
    } else {
        quote! {}
    };

    // Generate render statements for render_payload
    // Исключаем базовые поля — они не рендерятся в payload
    let render_statements: Vec<TokenStream> = fields
        .iter()
        .filter(|(field_name, _, _)| !is_base_field(&field_name.to_string()))
        .map(|(field_name, field_ty, field_attr)| {
            let inferred = InferredType::from_syn(field_ty)
                .expect("unsupported field type for render");
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());

            if let Some(fmt) = &field_attr.format {
                return quote! {
                    (#name_str, Some(::trace_parser::payload_template::TemplateValue::Str(
                        &::std::format!(#fmt, self.#field_name)
                    )))
                };
            }

            if field_attr.optional {
                let render = inferred.render_optional_value(field_name);
                quote! {
                    (#name_str, #render)
                }
            } else {
                let render = inferred.render_value(field_name);
                quote! {
                    (#name_str, #render)
                }
            }
        })
        .collect();

    // Generate parse statements for parse_payload
    // Исключаем базовые поля — они добавляются вручную из parts
    let parse_statements: Vec<TokenStream> = fields
        .iter()
        .filter(|(field_name, _, _)| !is_base_field(&field_name.to_string()))
        .map(|(field_name, field_ty, field_attr)| {
            let inferred = InferredType::from_syn(field_ty)
                .expect("unsupported field type for parse");
            let name_str = field_attr.name.clone()
                .unwrap_or_else(|| field_name.to_string());
            let regex = field_attr.regex.as_deref();

            if field_attr.optional {
                let parse = inferred.parse_optional_code(&quote! { captures }, &name_str, regex);
                quote! {
                    #field_name: #parse
                }
            } else {
                let parse = inferred.parse_code(&quote! { captures }, &name_str, regex);
                quote! {
                    #field_name: #parse
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

                    // Payload поля (все поля кроме базовых)
                    #(#parse_statements),*
                })
            }

            fn render_payload(&self) -> ::pyo3::PyResult<::std::string::String> {
                let template = Self::formats().template(self.format_id)
                    .ok_or_else(|| ::pyo3::exceptions::PyRuntimeError::new_err("No template found"))?;

                let values = &[
                    #(#render_statements),*
                ];

                template.format(values)
                    .ok_or_else(|| ::pyo3::exceptions::PyRuntimeError::new_err("Failed to format template"))
            }
        }

        #detect_format_override
    }
}

/// Generate registration code — regular, tracing_mark, or skipped
pub fn generate_registration(struct_name: &Ident, event_attr: &TraceEventAttr, is_tracing_mark: bool) -> TokenStream {
    if event_attr.skip_registration {
        return quote! {};
    }
    if is_tracing_mark {
        quote! {
            ::trace_parser::register_tracing_mark_parser!(#struct_name);
        }
    } else {
        let name = &event_attr.name;
        let aliases = &event_attr.aliases;
        let alias_registrations = aliases.iter().map(|alias| {
            quote! {
                ::trace_parser::register_parser!(#alias, #struct_name);
            }
        });
        quote! {
            ::trace_parser::register_parser!(#name, #struct_name);
            #(#alias_registrations)*
        }
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
                detect: vec![],
            })
            .collect();

        // Build fields
        let fields: Vec<(Ident, Type, FieldAttr)> = (0..ids.len())
            .map(|i| {
                (
                    syn::Ident::new(&format!("field{}", i), proc_macro2::Span::call_site()),
                    parse_quote!(u32),
                    FieldAttr {
                        name: None,
                        choice: vec![],
                        regex: None,
                        format: None,
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
            generate_pymethods: true,
            skip_registration: false,
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

        let output = generate_fast_match_impl(&struct_name, markers_attr.as_ref(), &[]);
        let output_str = output.to_string();

        assert!(output_str.contains("FastMatch"));
        assert!(output_str.contains("B"));
    }

    #[test]
    fn test_generate_fast_match_impl_empty() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let markers_attr: Option<TraceMarkersAttr> = None;

        let output = generate_fast_match_impl(&struct_name, markers_attr.as_ref(), &[]);
        let output_str = output.to_string();

        assert!(output_str.contains("FastMatch"));
    }

    #[test]
    fn test_generate_fast_match_impl_with_contains_any() {
        let struct_name: Ident = parse_quote!(TraceDevFrequency);
        let markers_attr: Option<TraceMarkersAttr> = None;
        let contains_any = vec!["clk=ddr_devfreq".to_string(), "clk=l3c_devfreq".to_string()];

        let output = generate_fast_match_impl(&struct_name, markers_attr.as_ref(), &contains_any);
        let output_str = output.to_string();

        assert!(output_str.contains("payload_quick_check"));
        assert!(output_str.contains("clk=ddr_devfreq"));
        assert!(output_str.contains("clk=l3c_devfreq"));
    }

    #[test]
    fn test_generate_registration() {
        let struct_name: Ident = parse_quote!(TraceSchedSwitch);
        let event_attr = TraceEventAttr {
            name: "sched_switch".to_string(),
            aliases: vec![],
            generate_pymethods: true,
            skip_registration: false,
        };

        let output = generate_registration(&struct_name, &event_attr, false);
        let output_str = output.to_string();

        assert!(output_str.contains("register_parser"));
        assert!(output_str.contains("TraceSchedSwitch"));
    }

    #[test]
    fn test_generate_registration_with_aliases() {
        let struct_name: Ident = parse_quote!(TraceExit);
        let event_attr = TraceEventAttr {
            name: "exit1".to_string(),
            aliases: vec!["exit2".to_string()],
            generate_pymethods: true,
            skip_registration: false,
        };

        let output = generate_registration(&struct_name, &event_attr, false);
        let output_str = output.to_string();

        assert!(output_str.contains("exit1"));
        assert!(output_str.contains("exit2"));
        assert!(output_str.contains("TraceExit"));
    }

    #[test]
    fn test_generate_registration_tracing_mark() {
        let struct_name: Ident = parse_quote!(TraceReceiveVsync);
        let event_attr = TraceEventAttr {
            name: "tracing_mark_write".to_string(),
            aliases: vec![],
            generate_pymethods: false,
            skip_registration: false,
        };

        let output = generate_registration(&struct_name, &event_attr, true);
        let output_str = output.to_string();

        assert!(output_str.contains("register_tracing_mark_parser"));
        assert!(output_str.contains("TraceReceiveVsync"));
    }

    #[test]
    fn test_generate_registration_skip_registration() {
        let struct_name: Ident = parse_quote!(TraceMarkBegin);
        let event_attr = TraceEventAttr {
            name: "tracing_mark_write".to_string(),
            aliases: vec![],
            generate_pymethods: false,
            skip_registration: true,
        };

        let output = generate_registration(&struct_name, &event_attr, true);
        assert_eq!(output.to_string(), quote! {}.to_string());
    }

    #[test]
    fn test_generate_template_event_impl_with_render() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(value),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
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
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(value),
            parse_quote!(Option<u32>),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
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
        let templates = vec![DefineTemplateAttr { template: "state={state}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(current_state),
            parse_quote!(u32),
            FieldAttr {
                name: Some("state".to_string()),
                choice: vec![],
                regex: None,
                format: None,
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
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(value),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
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
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(value),
            parse_quote!(Option<u32>),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
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
    fn test_generate_template_event_impl_with_parse_bool() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "flag={flag}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(flag),
            parse_quote!(bool),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
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
            DefineTemplateAttr { template: "a={a}".to_string(), id: Some(0), detect: vec![] },
            DefineTemplateAttr { template: "a={a} b={b}".to_string(), id: Some(1), detect: vec![] },
        ];
        let fields = vec![
            (parse_quote!(a), parse_quote!(u32), FieldAttr { name: None, choice: vec![], regex: None, format: None, optional: false, readonly: false, private: false }),
            (parse_quote!(b), parse_quote!(Option<u32>), FieldAttr { name: None, choice: vec![], regex: None, format: None, optional: true, readonly: false, private: false }),
        ];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        assert!(output_str.contains("detect_format"));
        assert!(output_str.contains("FormatSpec"));
    }

    #[test]
    fn test_generate_template_event_impl_single_template_detect_format() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "value={value}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(value),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Single template: detect_format calls detect_format_override
        assert!(output_str.contains("fn detect_format (payload"));
        assert!(output_str.contains("detect_format_override"));
        // detect_format_override inherent method is generated
        assert!(output_str.contains("fn detect_format_override (_payload"));
    }

    #[test]
    fn test_generate_template_event_impl_with_detect_markers() {
        let struct_name: Ident = parse_quote!(TraceSchedWakeup);
        let templates = vec![
            DefineTemplateAttr { template: "comm={comm} pid={pid}".to_string(), id: Some(0), detect: vec![] },
            DefineTemplateAttr { template: "comm={comm} pid={pid} reason={reason}".to_string(), id: Some(1), detect: vec!["reason=".to_string()] },
        ];
        let fields = vec![
            (parse_quote!(comm), parse_quote!(String), FieldAttr { name: None, choice: vec![], regex: None, format: None, optional: false, readonly: false, private: false }),
            (parse_quote!(pid), parse_quote!(u32), FieldAttr { name: None, choice: vec![], regex: None, format: None, optional: false, readonly: false, private: false }),
            (parse_quote!(reason), parse_quote!(Option<u32>), FieldAttr { name: None, choice: vec![], regex: None, format: None, optional: true, readonly: false, private: false }),
        ];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Should use SIMD markers, not detect_format_override
        assert!(output_str.contains("MARKERS"));
        assert!(output_str.contains("memmem :: find"));
        assert!(output_str.contains("reason"));
        assert!(!output_str.contains("detect_format_override"));
    }

    #[test]
    fn test_generate_template_event_impl_with_custom_regex() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "cpu={cpu_id}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(cpu_id),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: Some(r"\d{3}".to_string()),
                format: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Should use FieldSpec::custom with the regex
        assert!(output_str.contains("FieldSpec :: custom"));
        assert!(output_str.contains("\\d{3}"));
        // Parse via cap_str (not cap_parse) because custom regex
        assert!(output_str.contains("cap_str"));
    }

    #[test]
    fn test_generate_template_event_impl_with_format() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "target={target}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(target),
            parse_quote!(u32),
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: Some("{:03}".to_string()),
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Should use format! for rendering
        assert!(output_str.contains("format"));
        assert!(output_str.contains("03"));
        assert!(output_str.contains("TemplateValue :: Str"));
    }

    #[test]
    fn test_generate_template_event_impl_with_choice() {
        let struct_name: Ident = parse_quote!(TestEvent);
        let templates = vec![DefineTemplateAttr { template: "clk={clk}".to_string(), id: None, detect: vec![] }];
        let fields = vec![(
            parse_quote!(clk),
            parse_quote!(String),
            FieldAttr {
                name: None,
                choice: vec!["ddr_devfreq".to_string(), "l3c_devfreq".to_string()],
                regex: None,
                format: None,
                optional: false,
                readonly: false,
                private: false,
            },
        )];

        let output = generate_template_event_impl(&struct_name, &templates, &fields);
        let output_str = output.to_string();

        // Should use FieldSpec::choice
        assert!(output_str.contains("FieldSpec :: choice"));
        assert!(output_str.contains("ddr_devfreq"));
        assert!(output_str.contains("l3c_devfreq"));
    }
}
