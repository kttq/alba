use crate::{config::Language, sky::MoonPhaseName};

/// Required fields make a missing translation a compile error.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
    /// Sunday first.
    pub short_weekdays: [&'static str; 7],
    pub short_months: [&'static str; 12],
    /// In cycle order, from new moon.
    moon_phases: [&'static str; 8],
    /// Followed by `: ` and the configured name.
    pub location_not_found: &'static str,
    pub no_location: &'static str,
    pub fetching: &'static str,
    pub offline: &'static str,
    /// Around the whole hours of the age label: `2h ago`, `hace 2h`.
    pub age: [&'static str; 2],
}

impl Strings {
    pub const EN: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["sun", "mon", "tue", "wed", "thu", "fri", "sat"],
        short_months: [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ],
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
        no_location: "no location set",
        fetching: "fetching…",
        offline: "offline · retrying",
        age: ["", "h ago"],
    };
    pub const ES: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
        short_months: [
            "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
        ],
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
        no_location: "sin ubicación",
        fetching: "cargando…",
        offline: "offline · reintentando",
        age: ["hace ", "h"],
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
