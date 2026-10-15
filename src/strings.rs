use crate::{config::Language, sky::MoonPhaseName};

/// Required fields make a missing translation a compile error.
pub struct Strings {
    pub am: &'static str,
    pub pm: &'static str,
    /// Sunday first.
    pub short_weekdays: [&'static str; 7],
    pub short_months: [&'static str; 12],
    /// Calendar header names, Sunday first.
    pub weekday_initials: [&'static str; 7],
    /// Followed by a space and the ISO week number.
    pub iso_week: &'static str,
    /// Each greeting with the local hour it starts at, from the earliest.
    greetings: [(i8, &'static str); 4],
    /// Followed by a space and the time left.
    pub sunset_in: &'static str,
    pub sunrise_in: &'static str,
    /// In cycle order, from new moon.
    moon_phases: [&'static str; 8],
    /// Followed by `: ` and the configured name.
    pub location_not_found: &'static str,
    pub no_location: &'static str,
    pub fetching: &'static str,
    pub offline: &'static str,
    /// The text before and after the whole hours of the age label.
    pub age: [&'static str; 2],
    /// The first line of the first-run hint; the config file path follows on the second.
    pub first_run_hint: &'static str,
}

impl Strings {
    pub const EN: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["sun", "mon", "tue", "wed", "thu", "fri", "sat"],
        short_months: [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ],
        weekday_initials: ["su", "mo", "tu", "we", "th", "fr", "sa"],
        iso_week: "wk",
        greetings: [
            (5, "good morning ♥"),
            (12, "good afternoon ★"),
            (18, "good evening ★"),
            (23, "sleepy time ✦"),
        ],
        sunset_in: "sunset in",
        sunrise_in: "sunrise in",
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
        first_run_hint: "add location = \"your city\" to",
    };
    pub const ES: Self = Self {
        am: "am",
        pm: "pm",
        short_weekdays: ["dom", "lun", "mar", "mié", "jue", "vie", "sáb"],
        short_months: [
            "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
        ],
        weekday_initials: ["do", "lu", "ma", "mi", "ju", "vi", "sá"],
        iso_week: "sem",
        greetings: [
            (5, "buenos días ♥"),
            (12, "buenas tardes ★"),
            (20, "buenas noches ★"),
            (23, "a mimir ✦"),
        ],
        sunset_in: "atardece en",
        sunrise_in: "amanece en",
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
        first_run_hint: "agrega location = \"tu ciudad\" en",
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

    /// The greeting for a local hour, 0 to 23. Before the first start hour, the last greeting
    /// still holds from the evening.
    pub fn greeting(&self, hour: i8) -> &'static str {
        let (_, last) = self.greetings[self.greetings.len() - 1];
        (self.greetings.iter())
            .take_while(|(start, _)| *start <= hour)
            .last()
            .map_or(last, |(_, greeting)| greeting)
    }

    pub fn of(language: Language) -> &'static Self {
        match language {
            Language::En => &Self::EN,
            Language::Es => &Self::ES,
        }
    }
}
