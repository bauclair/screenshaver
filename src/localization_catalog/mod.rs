// Screenshaver factory localization catalog.
//
// Canonical English text and translator context live in keys.rs.
// Locale-specific translations live in one module per locale.
// Database synchronization mechanics remain in database_factory.rs.

#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryLanguage {
    pub locale: &'static str,
    pub english_name: &'static str,
    pub native_name: &'static str,
    pub text_direction: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryTranslationKey {
    pub key: &'static str,
    pub english_text: &'static str,
    pub translator_context: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FactoryTranslation {
    pub locale: &'static str,
    pub key: &'static str,
    pub translated_text: &'static str,
}

pub(crate) mod keys;
pub(crate) mod es_us;

pub(crate) const FACTORY_LANGUAGES: &[FactoryLanguage] = &[
    FactoryLanguage {
        locale: "en-US",
        english_name: "English (United States)",
        native_name: "English (United States)",
        text_direction: "ltr",
    },

    FactoryLanguage {
        locale: "es-US",
        english_name: "Spanish (United States)",
        native_name: "Español (Estados Unidos)",
        text_direction: "ltr",
    },
];