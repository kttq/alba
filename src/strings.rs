use crate::config::Language;

/// Every widget string, one constant per language. A struct literal must fill every field, so a
/// missing translation fails to compile.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
}

impl Strings {
    pub const EN: Self = Self { am: "am", pm: "pm" };
    pub const ES: Self = Self { am: "am", pm: "pm" };

    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::En => &Self::EN,
            Language::Es => &Self::ES,
        }
    }
}
