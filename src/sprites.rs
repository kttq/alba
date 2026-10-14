//! Const pixel art. Each sprite is rows of legend letters; `.` is an empty pixel.

use crate::pixels::{Legend, Pixel, PixelColor};

/// The two animation frames of an icon.
pub type Frames = [&'static [&'static str]; 2];

const CLOUD: Pixel = Pixel::new(PixelColor::Foreground);
const CLOUD_UNDERSIDE: Pixel = Pixel::new(PixelColor::BrightBlack);
const STORM_CLOUD: Pixel = Pixel::new(PixelColor::BrightBlack);
const SUN: Pixel = Pixel::new(PixelColor::Yellow);
const SUN_RAYS: Pixel = Pixel::new(PixelColor::BrightYellow);
const MOON: Pixel = Pixel::new(PixelColor::Yellow);
const RAIN: Pixel = Pixel::new(PixelColor::BrightBlue);
const SNOW: Pixel = Pixel::new(PixelColor::Foreground);
const FOG: Pixel = Pixel::new(PixelColor::Foreground);
const BACK_FOG: Pixel = Pixel::faint(PixelColor::Foreground);
const BOLT: Pixel = Pixel::new(PixelColor::Yellow);
const FLASH: Pixel = Pixel::new(PixelColor::BrightWhite);
const GHOST: Pixel = Pixel::faint(PixelColor::BrightBlack);

/// W cloud · w cloud underside · d storm cloud · Y sun · y sun rays · M moon · B rain · S snow ·
/// F fog · f back fog · L bolt
pub const ICON_LEGEND: &Legend = &[
    ('W', CLOUD),
    ('w', CLOUD_UNDERSIDE),
    ('d', STORM_CLOUD),
    ('Y', SUN),
    ('y', SUN_RAYS),
    ('M', MOON),
    ('B', RAIN),
    ('S', SNOW),
    ('F', FOG),
    ('f', BACK_FOG),
    ('L', BOLT),
];

/// The thunderstorm flash: the storm cloud turns bright white.
pub const FLASH_LEGEND: &Legend = &[('d', FLASH), ('L', BOLT)];

/// No weather data: every pixel of the cloudy icon in faint bright black.
pub const GHOST_LEGEND: &Legend = &[('W', GHOST), ('w', GHOST), ('d', GHOST)];

pub const FULL_CLEAR: Frames = [
    &[
        ".....yy.....",
        ".y........y.",
        "....YYYY....",
        "...YYYYYY...",
        "yy.YYYYYY.yy",
        "...YYYYYY...",
        "....YYYY....",
        ".y...yy...y.",
    ],
    &[
        ".....yy.....",
        "............",
        "....YYYY....",
        "...YYYYYY...",
        "y..YYYYYY..y",
        "...YYYYYY...",
        "....YYYY....",
        ".....yy.....",
    ],
];

pub const FULL_PARTLY_CLOUDY: Frames = [
    &[
        "..y.........",
        "....YYY.....",
        "y..YYYYY....",
        "...YYYWWW...",
        "..YYWWWWWWW.",
        "...WWWWWWWWW",
        "...wwwwwwww.",
        "............",
    ],
    &[
        "............",
        "....YYY.....",
        "...YYYYY....",
        "...YYYWWW...",
        "..YYWWWWWWW.",
        "...WWWWWWWWW",
        "...wwwwwwww.",
        "............",
    ],
];

const FULL_PARTLY_CLOUDY_NIGHT_FRAME: &[&str] = &[
    "....MMM.....",
    "...MM.......",
    "..MM........",
    "..MM..WWW...",
    "..MMWWWWWWW.",
    "...WWWWWWWWW",
    "...wwwwwwww.",
    "............",
];

/// Still: only its stars move.
pub const FULL_PARTLY_CLOUDY_NIGHT: Frames = [
    FULL_PARTLY_CLOUDY_NIGHT_FRAME,
    FULL_PARTLY_CLOUDY_NIGHT_FRAME,
];

pub const FULL_CLOUDY: Frames = [
    &[
        "............",
        ".......ddd..",
        "..WWW.ddddd.",
        ".WWWWWdddddd",
        "WWWWWWWWdddd",
        "WWWWWWWWWW..",
        ".wwwwwwwww..",
        "............",
    ],
    &[
        "............",
        "........ddd.",
        "..WWW..ddddd",
        ".WWWWWWddddd",
        "WWWWWWWWdddd",
        "WWWWWWWWWW..",
        ".wwwwwwwww..",
        "............",
    ],
];

pub const FULL_FOG: Frames = [
    &[
        ".FFFFFFFF...",
        "............",
        "...ffffffff.",
        "............",
        "FFFFFFF.....",
        "............",
        "..ffffffff..",
        "............",
    ],
    &[
        "..FFFFFFFF..",
        "............",
        "..ffffffff..",
        "............",
        ".FFFFFFF....",
        "............",
        ".ffffffff...",
        "............",
    ],
];

pub const FULL_DRIZZLE: Frames = [
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "...B.....B..",
        "............",
        "......B.....",
    ],
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "......B.....",
        "...B.....B..",
        "............",
    ],
];

pub const FULL_RAIN: Frames = [
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        ".B...B...B..",
        ".B...B...B..",
        "...B...B...B",
    ],
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "...B...B...B",
        ".B...B...B..",
        ".B...B...B..",
    ],
];

pub const FULL_HEAVY_RAIN: Frames = [
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "B.B.B.B.B.B.",
        "B.B.B.B.B.B.",
        ".B.B.B.B.B.B",
    ],
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        ".B.B.B.B.B.B",
        "B.B.B.B.B.B.",
        "B.B.B.B.B.B.",
    ],
];

pub const FULL_SNOW: Frames = [
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "............",
        ".S...S...S..",
        "...S...S...S",
    ],
    &[
        ".....WWW....",
        "..WW.WWWW...",
        ".WWWWWWWWWW.",
        "WWWWWWWWWWWW",
        ".wwwwwwwwww.",
        "............",
        "..S...S...S.",
        ".S...S...S..",
    ],
];

const FULL_THUNDERSTORM_FRAME: &[&str] = &[
    ".....ddd....",
    "..dd.dddd...",
    ".dddddddddd.",
    "dddddddddddd",
    ".dddddLdddd.",
    ".....LL.....",
    "......L.....",
    ".....L......",
];

/// Still: only the flash moves.
pub const FULL_THUNDERSTORM: Frames = [FULL_THUNDERSTORM_FRAME, FULL_THUNDERSTORM_FRAME];

pub const MINI_CLEAR: Frames = [
    &["..YY..", ".YYYY.", ".YYYY.", "..YY.."],
    &["y.YY.y", ".YYYY.", ".YYYY.", "y.YY.y"],
];

const MINI_PARTLY_CLOUDY_FRAME: &[&str] = &[".YY...", "YYWW..", ".WWWWW", "WWWWWW"];

pub const MINI_PARTLY_CLOUDY: Frames = [MINI_PARTLY_CLOUDY_FRAME, MINI_PARTLY_CLOUDY_FRAME];

const MINI_PARTLY_CLOUDY_NIGHT_FRAME: &[&str] = &[".MM...", "MMWW..", ".WWWWW", "WWWWWW"];

pub const MINI_PARTLY_CLOUDY_NIGHT: Frames = [
    MINI_PARTLY_CLOUDY_NIGHT_FRAME,
    MINI_PARTLY_CLOUDY_NIGHT_FRAME,
];

pub const MINI_CLOUDY: Frames = [
    &["...dd.", ".WWddd", "WWWWWd", ".wwww."],
    &["....dd", ".WW.dd", "WWWWWd", ".wwww."],
];

pub const MINI_FOG: Frames = [
    &["FFFFF.", "......", ".fffff", "......"],
    &[".FFFFF", "......", "fffff.", "......"],
];

pub const MINI_DRIZZLE: Frames = [
    &["..WW..", ".WWWW.", "WWWWWW", ".B..B."],
    &["..WW..", ".WWWW.", "WWWWWW", "...B.."],
];

pub const MINI_RAIN: Frames = [
    &["..WW..", ".WWWW.", "WWWWWW", "B.B.B."],
    &["..WW..", ".WWWW.", "WWWWWW", ".B.B.B"],
];

pub const MINI_HEAVY_RAIN: Frames = [
    &[".WWWW.", "WWWWWW", "B.B.B.", ".B.B.B"],
    &[".WWWW.", "WWWWWW", ".B.B.B", "B.B.B."],
];

pub const MINI_SNOW: Frames = [
    &["..WW..", ".WWWW.", "WWWWWW", "S..S.."],
    &["..WW..", ".WWWW.", "WWWWWW", ".S..S."],
];

const MINI_THUNDERSTORM_FRAME: &[&str] = &["..dd..", ".dddd.", "ddLddd", "..L..."];

pub const MINI_THUNDERSTORM: Frames = [MINI_THUNDERSTORM_FRAME, MINI_THUNDERSTORM_FRAME];

/// Night stars as (column, row) cells in the full and Large icons, with each star's glyph per
/// frame. A space is a star that is out in that frame.
pub const FULL_STARS: [(u16, u16); 4] = [(8, 0), (10, 1), (7, 0), (10, 3)];
pub const LARGE_STARS: [(u16, u16); 4] = [(17, 0), (21, 2), (14, 1), (21, 6)];
pub const STAR_GLYPHS: [[&str; 4]; 2] = [["✧", "✦", " ", "·"], ["✦", "·", "·", "✧"]];
