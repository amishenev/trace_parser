//! Generate Python .pyi stub fragments from Rust struct definitions.
//!
//! This module produces a string that can be assembled into a full `.pyi` file
//! by the `gen_stubs.py` script.

use crate::attrs::{FieldAttr, TraceEventAttr};
use std::path::PathBuf;
use syn::{Ident, Type};

/// Type mapping: Rust → Python
fn py_type(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => {
            let seg = tp.path.segments.last();
            match seg.map(|s| s.ident.to_string()).as_deref() {
                Some("String") => "str".into(),
                Some("u8") | Some("u16") | Some("u32") | Some("u64") | Some("i8") | Some("i16")
                | Some("i32") | Some("i64") => "int".into(),
                Some("f32") | Some("f64") => "float".into(),
                Some("bool") => "bool".into(),
                Some("Option") => {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.unwrap().arguments
                        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
                    {
                        format!("{} | None", py_type(inner))
                    } else {
                        "None".into()
                    }
                }
                _ => "typing.Any".into(),
            }
        }
        _ => "typing.Any".into(),
    }
}

/// Generate a single .pyi stub fragment for a struct.
/// Returns the string content (without imports).
pub fn generate_stub_string(
    struct_name: &Ident,
    fields: &[(Ident, Type, FieldAttr)],
    _event_attr: &TraceEventAttr,
) -> String {
    let mut out = String::new();

    let struct_name = struct_name.to_string();

    out.push_str(&format!("class {struct_name}:\n"));

    // 1. Class-level attributes (fields)
    for (field_name, field_ty, field_attr) in fields {
        if field_attr.private {
            continue;
        }
        let py_t = py_type(field_ty);
        out.push_str(&format!("    {field_name}: {py_t}\n"));
    }

    // 2. __init__
    out.push_str("    def __init__(self,\n");
    for (name, ty, attr) in fields {
        if attr.private {
            continue;
        }
        let py_t = py_type(ty);
        out.push_str(&format!("        {name}: {py_t},\n"));
    }
    out.push_str("    ) -> None: ...\n");

    // 3. Static methods
    out.push_str("    @staticmethod\n");
    out.push_str("    def can_be_parsed(line: str) -> bool: ...\n");
    out.push_str("    @staticmethod\n");
    out.push_str("    def parse(line: str) -> Self | None: ...\n");

    // 4. Properties
    out.push_str("    @property\n");
    out.push_str("    def payload(self) -> str: ...\n");
    out.push_str("    @property\n");
    out.push_str("    def template(self) -> str: ...\n");

    // 5. Instance methods
    out.push_str("    def to_string(self) -> str: ...\n");

    // 6. Dunder methods (last)
    out.push_str("    def __repr__(self) -> str: ...\n");
    out.push_str("    def __eq__(self, other: object) -> bool: ...\n");
    out.push_str("    def __str__(self) -> str: ...\n");
    out.push_str("    def __copy__(self) -> Self: ...\n");
    out.push_str("    def __deepcopy__(self, memo: object) -> Self: ...\n");

    out
}

/// Write the stub fragment to target/stubs/{struct_name}.stub
pub fn write_stub_file(
    struct_name: &Ident,
    fields: &[(Ident, Type, FieldAttr)],
    event_attr: &TraceEventAttr,
) {
    // Determine output directory: target/stubs/ relative to the crate root
    // or relative to CARGO_TARGET_DIR if set.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Try to find target dir relative to the macro crate
            // Fallback to workspace target
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("target")
        });

    let stub_dir = target_dir.join("stubs");
    if std::fs::create_dir_all(&stub_dir).is_err() {
        return; // Silently skip if we can't create the directory
    }

    let stub = generate_stub_string(struct_name, fields, event_attr);
    let stub_path = stub_dir.join(format!("{}.stub", struct_name));

    // Only write if content changed (avoid unnecessary rebuilds)
    let needs_write = match std::fs::read_to_string(&stub_path) {
        Ok(existing) => existing != stub,
        Err(_) => true,
    };

    if needs_write {
        let _ = std::fs::write(&stub_path, stub);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::FieldAttr;
    use syn::parse_quote;

    fn test_field(name: &str, ty: Type, private: bool, readonly: bool) -> (Ident, Type, FieldAttr) {
        (
            Ident::new(name, proc_macro2::Span::call_site()),
            ty,
            FieldAttr {
                name: None,
                choice: vec![],
                regex: None,
                format: None,
                readonly,
                private,
            },
        )
    }

    #[test]
    fn test_py_type_mapping() {
        assert_eq!(py_type(&parse_quote!(String)), "str");
        assert_eq!(py_type(&parse_quote!(u32)), "int");
        assert_eq!(py_type(&parse_quote!(f64)), "float");
        assert_eq!(py_type(&parse_quote!(bool)), "bool");
        assert_eq!(py_type(&parse_quote!(Option<u32>)), "int | None");
        assert_eq!(py_type(&parse_quote!(Option<bool>)), "bool | None");
    }

    #[test]
    fn test_generate_stub_string() {
        let fields = vec![
            test_field("thread_name", parse_quote!(String), false, false),
            test_field("thread_tid", parse_quote!(u32), false, false),
            test_field("format_id", parse_quote!(u8), true, false),
            test_field("event_name", parse_quote!(String), false, true),
        ];
        let event_attr = TraceEventAttr {
            name: "test_event".into(),
            aliases: vec![],
            generate_pymethods: true,
            skip_registration: false,
            mark_type: None,
        };
        let stub = generate_stub_string(&parse_quote!(TestEvent), &fields, &event_attr);

        assert!(stub.contains("class TestEvent:"));
        assert!(stub.contains("thread_name: str"));
        assert!(stub.contains("thread_tid: int"));
        assert!(!stub.contains("format_id")); // private
        assert!(stub.contains("event_name: str"));
        assert!(stub.contains("def can_be_parsed(line: str) -> bool"));
        assert!(stub.contains("def parse(line: str) -> Self | None"));
        assert!(stub.contains("def to_string(self) -> str"));
        assert!(stub.contains("def payload(self) -> str"));
        assert!(stub.contains("def template(self) -> str"));
        assert!(stub.contains("def __repr__(self) -> str"));
        assert!(stub.contains("def __copy__(self) -> Self"));
    }
}
