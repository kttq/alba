#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    HeavyRain,
    Snow,
    Thunderstorm,
}

impl Condition {
    /// Hail codes (96, 99) make no hail claim.
    pub fn from_wmo(code: u16) -> Option<Self> {
        let condition = match code {
            0 | 1 => Self::Clear,
            2 => Self::PartlyCloudy,
            3 => Self::Cloudy,
            45 | 48 => Self::Fog,
            51 | 53 | 55 | 56 | 57 => Self::Drizzle,
            61 | 63 | 66 | 80 | 81 => Self::Rain,
            65 | 67 | 82 => Self::HeavyRain,
            71 | 73 | 75 | 77 | 85 | 86 => Self::Snow,
            95 | 96 | 97 | 99 => Self::Thunderstorm,
            _ => return None,
        };
        Some(condition)
    }
}
