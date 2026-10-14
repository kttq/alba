use crate::{config::Language, sky::MoonPhaseName};

/// Required fields make a missing translation a compile error.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
    /// Sunday first.
    pub short_weekdays: [&'static str; 7],
    /// In `MoonPhaseName` order.
    moon_phases: [&'static str; 8],
}

impl Strings {
    pub const EN: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["sun", "mon", "tue", "wed", "thu", "fri", "sat"],
        moon_phases: [
            "new moon",
            "waxing crescent",
            "first quarter",
            "waxing gibbous",
            "full moon",
            "waning gibbous",
            "last quarter",
            "waning crescent",
        ],
    };
    pub const ES: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
        moon_phases: [
            "luna nueva",
            "luna creciente",
            "cuarto creciente",
            "gibosa creciente",
            "luna llena",
            "gibosa menguante",
            "cuarto menguante",
            "luna menguante",
        ],
    };

    pub fn moon_phase(&self, name: MoonPhaseName) -> &'static str {
        self.moon_phases[name as usize]
    }

    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::En => &Self::EN,
            Language::Es => &Self::ES,
        }
    }
}
