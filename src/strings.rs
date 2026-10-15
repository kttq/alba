use crate::{config::Language, sky::MoonPhaseName};

/// Required fields make a missing translation a compile error.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
    /// Sunday first.
    pub short_weekdays: [&'static str; 7],
    /// In cycle order, from new moon.
    moon_phases: [&'static str; 8],
    /// Followed by `: ` and the configured name.
    pub location_not_found: &'static str,
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
        location_not_found: "location not found",
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
        location_not_found: "ubicación no encontrada",
    };

    pub fn moon_phase(&self, name: MoonPhaseName) -> &'static str {
        let index = match name {
            MoonPhaseName::NewMoon => 0,
            MoonPhaseName::WaxingCrescent => 1,
            MoonPhaseName::FirstQuarter => 2,
            MoonPhaseName::WaxingGibbous => 3,
            MoonPhaseName::FullMoon => 4,
            MoonPhaseName::WaningGibbous => 5,
            MoonPhaseName::LastQuarter => 6,
            MoonPhaseName::WaningCrescent => 7,
        };
        self.moon_phases[index]
    }

    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::En => &Self::EN,
            Language::Es => &Self::ES,
        }
    }
}
