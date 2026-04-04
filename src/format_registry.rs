use crate::payload_template::PayloadTemplate;

/// Спецификация одного формата payload
pub struct FormatSpec {
    pub kind: u8,
    pub template: &'static PayloadTemplate,
}

/// Словарь форматов для события
pub struct FormatRegistry {
    pub(crate) formats: &'static [FormatSpec],
}

impl FormatRegistry {
    /// Создать registry из массива спецификаций
    /// Использует `Box::leak` для создания `'static` ссылки
    pub fn new(formats: Vec<FormatSpec>) -> Self {
        let leaked = Box::leak(formats.into_boxed_slice());
        Self { formats: leaked }
    }

    /// Получить шаблон по индексу формата
    pub fn template(&self, kind: u8) -> Option<&'static PayloadTemplate> {
        self.formats
            .iter()
            .find(|spec| spec.kind == kind)
            .map(|spec| spec.template)
    }
}
