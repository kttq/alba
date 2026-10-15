mod common;

use alba::{
    icon::{Icon, IconSize},
    motion::Motion,
    sky::MoonPhase,
    weather::Condition,
};
use common::{HOUR, at, canvas_snapshot, canvas_text, pixel_snapshot};
use jiff::Zoned;
use ratatui::{buffer::Buffer, layout::Rect, style::Style};

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

fn frame_one() -> Motion {
    Motion {
        colon_lit: true,
        icon_frame: 0,
        flash: false,
    }
}

fn frame_two() -> Motion {
    Motion {
        icon_frame: 1,
        ..frame_one()
    }
}

fn flash() -> Motion {
    Motion {
        flash: true,
        ..frame_one()
    }
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

fn rendered(icon: Icon, size: IconSize, motion: Motion) -> String {
    let (width, height) = size.cells();
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    icon.render(size, motion, area, &mut buffer);
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
fn frames_differ_except_where_identical_by_design() {
    for (name, icon) in every_icon() {
        for (size_name, size) in SIZES {
            let frames_differ =
                rendered(icon, size, frame_one()) != rendered(icon, size, frame_two());
            assert_eq!(
                frames_differ,
                !identical_by_design(icon, size),
                "{name} {size_name}"
            );
        }
    }
}

/// The storm moves only through its flash, the ghost stands still, and mini icons have no stars.
fn identical_by_design(icon: Icon, size: IconSize) -> bool {
    matches!(
        (icon, size),
        (Icon::Day(Condition::Thunderstorm) | Icon::Ghost, _)
            | (
                Icon::Day(Condition::PartlyCloudy)
                    | Icon::Night(Condition::Clear | Condition::PartlyCloudy, _),
                IconSize::Mini,
            )
    )
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
            for motion in [frame_one(), frame_two(), flash()] {
                let canvas = icon.canvas(size, motion);
                assert_eq!(
                    canvas_snapshot(&canvas),
                    canvas_text(&canvas),
                    "{name} {size:?}"
                );
            }
        }
    }
}

#[test]
fn stars_draw_in_the_default_foreground_over_any_style() {
    let icon = Icon::Night(Condition::Clear, tonight());
    let (width, height) = IconSize::Full.cells();
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    buffer.set_style(area, Style::new().red().on_blue().dim());

    icon.render(IconSize::Full, frame_one(), area, &mut buffer);

    let top_row = pixel_snapshot(&buffer, area)
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert_eq!(top_row, "......K~K~K~yy..✧f......");
}
