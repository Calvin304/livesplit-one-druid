use druid::{Color, FontDescriptor, FontFamily, FontWeight};

pub const ICON_SIZE: f64 = 140.0;
pub const MARGIN: f64 = 20.0;
pub const SPACING: f64 = 16.0;
pub const BUTTON_SPACING: f64 = 8.0;
pub const BUTTON_HEIGHT: f64 = 30.0;
pub const GRID_BORDER: f64 = 4.0;
pub const DIALOG_BUTTON_WIDTH: f64 = 80.0;
pub const DIALOG_BUTTON_HEIGHT: f64 = 35.0;
pub const KEY_FONT: FontDescriptor = FontDescriptor::new(FontFamily::SYSTEM_UI)
    .with_weight(FontWeight::BOLD)
    .with_size(14.0);
pub const COLUMN_LABEL_FONT: FontDescriptor = KEY_FONT;
pub const TABLE_HORIZONTAL_MARGIN: f64 = 10.0;
pub const TIME_COLUMN_WIDTH: f64 = 110.0;
pub const ATTEMPTS_OFFSET_WIDTH: f64 = 140.0;

pub const SELECTED_TEXT_BACKGROUND_COLOR: Color = Color::rgb8(5, 99, 212);
pub const BUTTON_TOP: Color = Color::grey8(0x1c);
pub const BUTTON_BOTTOM: Color = Color::grey8(0x12);
pub const BACKGROUND: Color = Color::grey8(0x1c);
pub const BUTTON_BORDER: Color = Color::grey8(0x40);
pub const TEXTBOX_BACKGROUND: Color = Color::grey8(0x27);
pub const PRIMARY_LIGHT: Color = Color::grey8(0x80);
pub const BUTTON_BORDER_RADIUS: f64 = 5.0;
