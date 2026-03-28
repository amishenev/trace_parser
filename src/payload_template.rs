use std::collections::HashMap;

use regex::Regex;

pub struct PayloadTemplate {
    regex: Regex,
    segments: Vec<TemplateSegment>,
}

enum TemplateSegment {
    Literal(String),
    Field(String),
    IgnoredField,
    Whitespace,
    OptionalWhitespace,
}

pub enum TemplateValue<'a> {
    Str(&'a str),
    U32(u32),
    I32(i32),
    F64(f64),
    BoolInt(bool),
}

pub struct FieldSpec {
    regex_fragment: String,
}

impl FieldSpec {
    pub fn string() -> Self {
        Self::custom(r".+?")
    }

    pub fn u32() -> Self {
        Self::custom(r"\d+")
    }

    pub fn i32() -> Self {
        Self::custom(r"-?\d+")
    }

    pub fn f64() -> Self {
        Self::custom(r"-?\d+(?:\.\d+)?")
    }

    pub fn bool_int() -> Self {
        Self::custom(r"[01]")
    }

    pub fn choice(options: &[&str]) -> Self {
        let pattern = options
            .iter()
            .map(|item| regex::escape(item))
            .collect::<Vec<_>>()
            .join("|");
        Self::custom(&format!("(?:{pattern})"))
    }

    pub fn custom(regex_fragment: &str) -> Self {
        Self {
            regex_fragment: regex_fragment.to_owned(),
        }
    }
}

impl PayloadTemplate {
    pub fn new(template: &str, fields: &[(&str, FieldSpec)]) -> Self {
        let field_map = fields
            .iter()
            .map(|(name, spec)| (*name, spec))
            .collect::<HashMap<_, _>>();

        let mut regex_pattern = String::from("^");
        let mut segments = Vec::new();
        let mut chars = template.chars().peekable();
        let mut literal = String::new();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if !literal.is_empty() {
                    regex_pattern.push_str(&regex::escape(&literal));
                    segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                }

                let mut field_name = String::new();
                while let Some(next) = chars.next() {
                    if next == '}' {
                        break;
                    }
                    field_name.push(next);
                }

                // Built-in service tokens let templates normalize whitespace without
                // polluting the typed payload with extra fields.
                match field_name.as_str() {
                    "ws" => {
                        regex_pattern.push_str(r"\s+");
                        segments.push(TemplateSegment::Whitespace);
                    }
                    "?ws" => {
                        regex_pattern.push_str(r"\s*");
                        segments.push(TemplateSegment::OptionalWhitespace);
                    }
                    _ if field_name.starts_with("ignore:") => {
                        let ignored_name = &field_name["ignore:".len()..];
                        let regex_fragment = field_map
                            .get(ignored_name)
                            .unwrap_or_else(|| panic!("missing field spec for `{ignored_name}`"));
                        regex_pattern.push_str(&format!(
                            "(?P<{}>{})",
                            ignored_name, regex_fragment.regex_fragment
                        ));
                        segments.push(TemplateSegment::IgnoredField);
                    }
                    _ if field_name.starts_with("?ignore:") => {
                        let ignored_name = &field_name["?ignore:".len()..];
                        let regex_fragment = field_map
                            .get(ignored_name)
                            .unwrap_or_else(|| panic!("missing field spec for `{ignored_name}`"));
                        regex_pattern.push_str(&format!(
                            "(?P<{}>{})?",
                            ignored_name, regex_fragment.regex_fragment
                        ));
                        segments.push(TemplateSegment::IgnoredField);
                    }
                    _ => {
                        let regex_fragment = field_map
                            .get(field_name.as_str())
                            .unwrap_or_else(|| panic!("missing field spec for `{field_name}`"));
                        regex_pattern.push_str(&format!(
                            "(?P<{}>{})",
                            field_name, regex_fragment.regex_fragment
                        ));
                        segments.push(TemplateSegment::Field(field_name));
                    }
                }
            } else {
                literal.push(ch);
            }
        }

        if !literal.is_empty() {
            regex_pattern.push_str(&regex::escape(&literal));
            segments.push(TemplateSegment::Literal(literal));
        }

        regex_pattern.push('$');

        Self {
            regex: Regex::new(&regex_pattern).expect("payload template regex must compile"),
            segments,
        }
    }

    pub fn is_match(&self, input: &str) -> bool {
        self.regex.is_match(input)
    }

    pub fn captures<'a>(&self, input: &'a str) -> Option<regex::Captures<'a>> {
        self.regex.captures(input)
    }

    pub fn format(&self, values: &HashMap<&str, TemplateValue<'_>>) -> Option<String> {
        let mut rendered = String::new();

        for segment in &self.segments {
            match segment {
                TemplateSegment::Literal(text) => rendered.push_str(text),
                TemplateSegment::Field(name) => {
                    let value = values.get(name.as_str());
                    if let Some(value) = value {
                        push_value(&mut rendered, value);
                    }
                }
                TemplateSegment::IgnoredField => {}
                // `{ws}` is canonicalized to a single space on output.
                TemplateSegment::Whitespace => rendered.push(' '),
                // `{?ws}` accepts optional whitespace during parsing and disappears on output.
                TemplateSegment::OptionalWhitespace => {}
            }
        }

        Some(rendered)
    }
}

fn push_value(output: &mut String, value: &TemplateValue<'_>) {
    match value {
        TemplateValue::Str(text) => output.push_str(text),
        TemplateValue::U32(number) => output.push_str(&number.to_string()),
        TemplateValue::I32(number) => output.push_str(&number.to_string()),
        TemplateValue::F64(number) => output.push_str(&number.to_string()),
        TemplateValue::BoolInt(value) => output.push_str(if *value { "1" } else { "0" }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::payload_template::{FieldSpec, PayloadTemplate, TemplateValue};

    #[test]
    fn service_ws_matches_and_normalizes_to_single_space() {
        let template = PayloadTemplate::new(
            "a={left}{ws}b={right}",
            &[("left", FieldSpec::string()), ("right", FieldSpec::u32())],
        );
        assert!(template.is_match("a=x   b=42"));

        let rendered = template
            .format(&HashMap::from([
                ("left", TemplateValue::Str("x")),
                ("right", TemplateValue::U32(42)),
            ]))
            .expect("template must render");

        assert_eq!(rendered, "a=x b=42");
    }

    #[test]
    fn service_optional_ws_can_disappear_on_output() {
        let template = PayloadTemplate::new(
            "a={left}{?ws}b={right}",
            &[("left", FieldSpec::string()), ("right", FieldSpec::u32())],
        );
        assert!(template.is_match("a=xb=42"));
        assert!(template.is_match("a=x   b=42"));

        let rendered = template
            .format(&HashMap::from([
                ("left", TemplateValue::Str("x")),
                ("right", TemplateValue::U32(42)),
            ]))
            .expect("template must render");

        assert_eq!(rendered, "a=xb=42");
    }

    #[test]
    fn ignored_field_matches_but_is_not_rendered() {
        let template = PayloadTemplate::new(
            "{?ignore:ignored}ReceiveVsync {frame}",
            &[
                ("ignored", FieldSpec::custom(r"\[[^]]+\]")),
                ("frame", FieldSpec::u32()),
            ],
        );

        assert!(template.is_match("[ExtraInfo]ReceiveVsync 42"));
        assert!(template.is_match("ReceiveVsync 42"));

        let rendered = template
            .format(&HashMap::from([("frame", TemplateValue::U32(42))]))
            .expect("template must render");

        assert_eq!(rendered, "ReceiveVsync 42");
    }
}
