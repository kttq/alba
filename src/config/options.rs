use serde::Deserialize;
use serde_repr::Deserialize_repr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    #[default]
    Metric,
    Imperial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize_repr)]
#[repr(u8)]
pub enum HourFormat {
    #[default]
    TwentyFour = 24,
    Twelve = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}
