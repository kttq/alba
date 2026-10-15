//! PROTOTYPE (issue #55), throwaway: the revamped look. The v1 face with the week in place of
//! the month, laid out on a size ladder (`layout::ladder`), the 24-bit icons where the terminal
//! has 24-bit color, and three themes. Keys: h theme, t/T time ±1h, c condition, b bar. Env:
//! ALBA_LOOK_THEME=terminal|dark|warm, ALBA_LOOK_TIME=HH:MM, ALBA_LOOK_CONDITION=rain,
//! ALBA_LOOK_TICK=n freezes the motion at one quarter-second tick, ALBA_LOOK_BAR=0 hides the bar.

mod icons;
pub mod probe;
mod rgb;
mod theme;

use jiff::{SignedDuration, Zoned};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Widget,
};

use self::{rgb::Px, theme::Theme};
use crate::{
    config::Config,
    face::Face,
    icon::{Icon, IconSize},
    layout::{
        Piece,
        ladder::{self, Week},
    },
    strings::Strings,
    weather::Condition,
};

const CONDITIONS: [Option<Condition>; 10] = [
    None,
    Some(Condition::Clear),
    Some(Condition::PartlyCloudy),
    Some(Condition::Cloudy),
    Some(Condition::Fog),
    Some(Condition::Drizzle),
    Some(Condition::Rain),
    Some(Condition::HeavyRain),
    Some(Condition::Snow),
    Some(Condition::Thunderstorm),
];

pub struct PrototypeLook {
    config: Config,
    /// The 24-bit icons in place of v1's, when the terminal has 24-bit color.
    rich: bool,
    theme: Theme,
    time_offset: SignedDuration,
    /// An index into `CONDITIONS`; 0 shows the real weather.
    condition: usize,
    bar: bool,
}

impl PrototypeLook {
    pub fn new(config: Config, profile: &probe::Profile) -> Self {
        let env = |name: &str| std::env::var(name).ok();
        let theme = env("ALBA_LOOK_THEME")
            .and_then(|key| Theme::ALL.into_iter().find(|theme| theme.key() == key))
            .unwrap_or(Theme::Terminal);
        let time_offset = env("ALBA_LOOK_TIME")
            .and_then(|text| {
                let (hour, minute) = text.split_once(':')?;
                let (hour, minute): (i64, i64) = (hour.parse().ok()?, minute.parse().ok()?);
                let now = Zoned::now();
                let now_minutes = i64::from(now.hour()) * 60 + i64::from(now.minute());
                Some(SignedDuration::from_mins(hour * 60 + minute - now_minutes))
            })
            .unwrap_or(SignedDuration::ZERO);
        let condition = env("ALBA_LOOK_CONDITION")
            .and_then(|name| {
                CONDITIONS.iter().position(|condition| {
                    condition.is_some_and(|condition| {
                        format!("{condition:?}").eq_ignore_ascii_case(&name.replace(['-', '_'], ""))
                    })
                })
            })
            .unwrap_or(0);
        Self {
            config,
            rich: profile.truecolor(),
            theme,
            time_offset,
            condition,
            bar: profile.answered && env("ALBA_LOOK_BAR").as_deref() != Some("0"),
        }
    }

    /// True when the key was the prototype's. Keys with ctrl stay alba's, so ctrl-c quits.
    pub fn key(&mut self, event: KeyEvent) -> bool {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }
        match event.code {
            KeyCode::Char('h') => {
                let index = Theme::ALL
                    .iter()
                    .position(|theme| *theme == self.theme)
                    .unwrap_or(0);
                self.theme = Theme::ALL[(index + 1) % Theme::ALL.len()];
            }
            KeyCode::Char('t') => self.time_offset += SignedDuration::from_hours(1),
            KeyCode::Char('T') => self.time_offset -= SignedDuration::from_hours(1),
            KeyCode::Char('c') => self.condition = (self.condition + 1) % CONDITIONS.len(),
            KeyCode::Char('b') => self.bar = !self.bar,
            _ => return false,
        }
        true
    }

    pub fn display_now(&self, now: &Zoned) -> Zoned {
        now.checked_add(self.time_offset)
            .unwrap_or_else(|_| now.clone())
    }

    pub fn render(&self, area: Rect, buffer: &mut Buffer, face: &Face, now: &Zoned) {
        let week = Week::at(
            self.config.week_start,
            Strings::of(self.config.language),
            now.date(),
        );
        let (report, rung) = ladder::lay_out(area, face, &week);
        for item in report.items {
            match item.piece {
                Piece::Icon(shown, icon_size @ (IconSize::Large | IconSize::Full)) if self.rich => {
                    self.draw_icon(shown.icon, icon_size, item.area, buffer, now);
                }
                piece => piece.render(item.area, buffer),
            }
        }
        theme::accents(self.theme, area, buffer);
        theme::paint(self.theme, area, buffer);
        if self.bar && area.height >= 2 {
            let rung = rung.map_or("below the ladder", ladder::Rung::name);
            let condition = CONDITIONS[self.condition].map_or("real".to_string(), |condition| {
                format!("{condition:?}").to_lowercase()
            });
            let text = format!(
                " {} · {rung} · {:02}:{:02} · {condition} · h t/T c b ",
                self.theme.key(),
                now.hour(),
                now.minute()
            );
            let line = Line::styled(
                text,
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
            let width = (line.width() as u16).min(area.width);
            line.render(
                Rect::new(
                    area.x + (area.width - width) / 2,
                    area.bottom() - 1,
                    width,
                    1,
                ),
                buffer,
            );
        }
    }

    /// The 24-bit icon, with the condition from the `c` key when it is set.
    fn draw_icon(
        &self,
        icon: Icon,
        icon_size: IconSize,
        area: Rect,
        buffer: &mut Buffer,
        now: &Zoned,
    ) {
        let icon = match (CONDITIONS[self.condition], icon) {
            (None, icon) => icon,
            (Some(condition), Icon::Night(_, moon_phase)) => Icon::Night(condition, moon_phase),
            (Some(condition), _) => Icon::Day(condition),
        };
        let tick = self.config.motion.then(|| {
            std::env::var("ALBA_LOOK_TICK")
                .ok()
                .and_then(|tick| tick.parse().ok())
                .unwrap_or_else(|| now.timestamp().as_millisecond().div_euclid(250))
        });
        let (width, height) = icon_size.cells();
        let (width, height) = (i32::from(width), i32::from(height) * 2);
        let mut px = Px::new(width, height);
        icons::icon(&mut px, width, height, icon, tick);
        // The icons were drawn for a near-black screen; their dark halo would box them in.
        for y in 0..height {
            for x in 0..width {
                if px.get(x, y).is_some_and(|color| color.luma() < 0.08) {
                    px.clear(x, y);
                }
            }
        }
        px.render_half(area, buffer);
    }
}
