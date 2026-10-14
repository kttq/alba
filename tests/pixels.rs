mod common;

use alba::pixels::{Canvas, Legend, Pixel, PixelColor};
use common::pixel_snapshot;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

const LEGEND: &Legend = &[
    ('f', Pixel::new(PixelColor::Foreground)),
    ('y', Pixel::new(PixelColor::Yellow)),
    ('B', Pixel::new(PixelColor::BrightBlue)),
    ('K', Pixel::faint(PixelColor::BrightBlack)),
    ('r', Pixel::faint(PixelColor::Red)),
];

fn rendered(canvas: &Canvas) -> String {
    let area = Rect::new(0, 0, canvas.width(), canvas.height().div_ceil(2));
    let mut buffer = Buffer::empty(area);
    canvas.render(area, &mut buffer);
    pixel_snapshot(&buffer, area)
}

#[test]
fn each_cell_packs_a_top_and_a_bottom_pixel() {
    let canvas = Canvas::from_rows(&["fy.fyK", "f.yByy"], LEGEND);

    assert_eq!(
        rendered(&canvas),
        "ffyy..ffyyK~\n\
         ff..yyBByyyy\n"
    );
}

#[test]
fn the_default_foreground_always_takes_the_foreground_glyph() {
    let canvas = Canvas::from_rows(&["yK", "ff"], LEGEND);

    assert_eq!(rendered(&canvas), "yyKK\nffff\n");
}

#[test]
fn two_faint_pixels_of_different_colors_draw_the_bottom_at_full_strength() {
    let canvas = Canvas::from_rows(&["K", "r"], LEGEND);

    assert_eq!(rendered(&canvas), "K~\nrr\n");
}

#[test]
fn an_odd_height_leaves_an_empty_bottom_pixel() {
    let canvas = Canvas::from_rows(&["fy", "yf", "Bf"], LEGEND);

    assert_eq!(canvas.height(), 3);
    assert_eq!(rendered(&canvas), "ffyy\nyyff\nBBff\n....\n");
}

#[test]
fn a_painted_canvas_overwrites_only_its_lit_pixels() {
    let mut canvas = Canvas::from_rows(&["yyy", "yyy"], LEGEND);

    canvas.paint(1, 1, &Canvas::from_rows(&["B.", ".B"], LEGEND));

    assert_eq!(rendered(&canvas), "yyyyyy\nyyBByy\n");
}

#[test]
fn painting_past_the_edge_is_clipped() {
    let mut canvas = Canvas::new(2, 2);

    canvas.paint(1, 1, &Canvas::from_rows(&["BB", "BB"], LEGEND));

    assert_eq!(rendered(&canvas), "....\n..BB\n");
}

#[test]
fn scaled_makes_each_pixel_a_square_block() {
    let canvas = Canvas::from_rows(&["fy", "K."], LEGEND).scaled(2);

    assert_eq!(
        rendered(&canvas),
        "ffffyyyy\n\
         ffffyyyy\n\
         K~K~....\n\
         K~K~....\n"
    );
}

#[test]
fn glyphs_decode_as_themselves_and_their_color() {
    let area = Rect::new(0, 0, 2, 1);
    let mut buffer = Buffer::empty(area);
    buffer[(1, 0)].set_symbol("✦");

    assert_eq!(pixel_snapshot(&buffer, area), "..✦f\n..✦f\n");
}
