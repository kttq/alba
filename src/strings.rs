/// Every widget string, one constant per language. A struct literal must fill every field, so a
/// missing translation fails to compile.
pub struct Strings {}

impl Strings {
    pub const EN: Self = Self {};
    pub const ES: Self = Self {};
}
