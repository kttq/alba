use crate::config::Language;

/// Required fields make a missing translation a compile error.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
    /// Sunday first.
    pub short_weekdays: [&'static str; 7],
}

impl Strings {
    pub const EN: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["sun", "mon", "tue", "wed", "thu", "fri", "sat"],
    };
    pub const ES: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
    };

    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::En => &Self::EN,
            Language::Es => &Self::ES,
        }
    }
}
