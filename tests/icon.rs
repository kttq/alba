mod common;

use alba::{
    icon::{Animation, Icon, IconSize},
    pixels::Canvas,
    sky::MoonPhase,
    weather::Condition,
};
use common::{HOUR, at, pixel_snapshot};
use jiff::Zoned;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

const SIZES: [(&str, IconSize); 3] = [
    ("mini", IconSize::Mini),
    ("full", IconSize::Full),
    ("large", IconSize::Large),
];

fn instant(time: &str) -> Zoned {
    format!("2026-10-13T{time}[America/Argentina/Buenos_Aires]")
        .parse()
        .unwrap()
}

/// Away from the flash, which lights in the first quarter second of every eighth second.
fn frame_one() -> Animation {
    Animation::at(&instant("06:38:02.5"), true)
}

fn frame_two() -> Animation {
    Animation::at(&instant("06:38:03.5"), true)
}

fn flash() -> Animation {
    Animation::at(&instant("06:38:00"), true)
}

fn tonight() -> MoonPhase {
    MoonPhase::at(instant("06:38").timestamp())
}

fn every_icon() -> Vec<(&'static str, Icon)> {
    let day = [
        ("clear", Condition::Clear),
        ("partly_cloudy", Condition::PartlyCloudy),
        ("cloudy", Condition::Cloudy),
        ("fog", Condition::Fog),
        ("drizzle", Condition::Drizzle),
        ("rain", Condition::Rain),
        ("heavy_rain", Condition::HeavyRain),
        ("snow", Condition::Snow),
        ("thunderstorm", Condition::Thunderstorm),
    ];
    let mut icons: Vec<_> = day
        .into_iter()
        .map(|(name, condition)| (name, Icon::Day(condition)))
        .collect();
    icons.push(("clear_night", Icon::Night(Condition::Clear, tonight())));
    icons.push((
        "partly_cloudy_night",
        Icon::Night(Condition::PartlyCloudy, tonight()),
    ));
    icons.push(("ghost", Icon::Ghost));
    icons
}

fn rendered(icon: Icon, size: IconSize, animation: Animation) -> String {
    let (width, height) = size.cells();
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    icon.render(size, animation, area, &mut buffer);
    pixel_snapshot(&buffer, area)
}

fn rendered_canvas(canvas: &Canvas) -> String {
    let area = Rect::new(0, 0, canvas.width(), canvas.height().div_ceil(2));
    let mut buffer = Buffer::empty(area);
    canvas.render(area, &mut buffer);
    pixel_snapshot(&buffer, area)
}

#[test]
fn every_icon_in_every_size_and_frame() {
    for (name, icon) in every_icon() {
        for (size_name, size) in SIZES {
            let snapshot = format!(
                "frame 1\n{}frame 2\n{}",
                rendered(icon, size, frame_one()),
                rendered(icon, size, frame_two())
            );
            insta::assert_snapshot!(format!("{name}_{size_name}"), snapshot);
        }
    }
}

#[test]
fn the_thunderstorm_flash_in_every_size() {
    for (size_name, size) in SIZES {
        insta::assert_snapshot!(
            format!("thunderstorm_flash_{size_name}"),
            rendered(Icon::Day(Condition::Thunderstorm), size, flash())
        );
    }
}

#[test]
fn the_flash_lasts_one_tick_at_the_start_of_each_eight_seconds() {
    let storm = |time: &str, motion: bool| {
        let animation = Animation::at(&instant(time), motion);
        rendered(
            Icon::Day(Condition::Thunderstorm),
            IconSize::Full,
            animation,
        )
    };
    let flashing = storm("06:38:00", true);
    let still = storm("06:38:02.5", true);

    assert_ne!(flashing, still);
    assert_eq!(storm("06:38:08.249", true), flashing);
    assert_eq!(storm("06:38:08.25", true), still);
    assert_eq!(storm("06:38:04", true), still);
    assert_eq!(storm("06:38:00", false), still);
}

#[test]
fn frames_change_every_second_and_stop_without_motion() {
    let rain = |time: &str, motion: bool| {
        let animation = Animation::at(&instant(time), motion);
        rendered(Icon::Day(Condition::Rain), IconSize::Full, animation)
    };

    assert_eq!(rain("06:38:02.5", true), rain("06:38:04", true));
    assert_eq!(rain("06:38:03.5", true), rain("06:38:05", true));
    assert_ne!(rain("06:38:02.5", true), rain("06:38:03.5", true));
    assert_eq!(rain("06:38:03.5", false), rain("06:38:02.5", true));
}

#[test]
fn frames_differ_except_where_identical_by_design() {
    let identical_by_design = [
        "thunderstorm_mini",
        "thunderstorm_full",
        "thunderstorm_large",
        "partly_cloudy_mini",
        "partly_cloudy_night_mini",
        "clear_night_mini",
        "ghost_mini",
        "ghost_full",
        "ghost_large",
    ];
    for (name, icon) in every_icon() {
        for (size_name, size) in SIZES {
            let label = format!("{name}_{size_name}");
            let frames_differ =
                rendered(icon, size, frame_one()) != rendered(icon, size, frame_two());
            assert_eq!(
                frames_differ,
                !identical_by_design.contains(&label.as_str()),
                "{label}"
            );
        }
    }
}

#[test]
fn large_is_the_full_icon_at_twice_the_scale() {
    for (name, icon) in every_icon() {
        assert_eq!(
            icon.canvas(IconSize::Large, frame_two()),
            icon.canvas(IconSize::Full, frame_two()).scaled(2),
            "{name}"
        );
    }
}

#[test]
fn every_painted_pixel_survives_the_cell_packing() {
    let mut icons = every_icon();
    for hour in (0..30 * 24).step_by(6) {
        let moon_phase = MoonPhase::at(at(1_791_849_600 + hour * HOUR));
        icons.push(("clear night", Icon::Night(Condition::Clear, moon_phase)));
    }
    for (name, icon) in icons {
        for (_, size) in SIZES {
            for animation in [frame_one(), frame_two(), flash()] {
                let canvas = icon.canvas(size, animation);
                assert_eq!(
                    rendered_canvas(&canvas),
                    painted(&canvas),
                    "{name} {size:?}"
                );
            }
        }
    }
}

/// The canvas in the pixel snapshot alphabet, read straight from its pixels.
fn painted(canvas: &Canvas) -> String {
    let height = canvas.height().div_ceil(2) * 2;
    (0..height)
        .map(|y| {
            let row: String = (0..canvas.width())
                .map(|x| match canvas.pixel(x, y) {
                    None => "..".to_string(),
                    Some(pixel) => {
                        let letter = common::color_letter(pixel.color.into());
                        if pixel.faint {
                            format!("{letter}~")
                        } else {
                            format!("{letter}{letter}")
                        }
                    }
                })
                .collect();
            row + "\n"
        })
        .collect()
}
