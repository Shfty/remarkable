use crate::{
    display::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
    ROWS, ROW_HEIGHT,
};
use libremarkable::framebuffer::common::mxcfb_rect as MxcfbRect;

pub const PANEL_HEIGHT: i32 = ROW_HEIGHT as i32 * ROWS as i32;

pub const PANEL_RECT: MxcfbRect = MxcfbRect {
    left: 0,
    top: (DISPLAY_HEIGHT as u32 - PANEL_HEIGHT as u32) as u32,
    width: DISPLAY_WIDTH as u32,
    height: PANEL_HEIGHT as u32,
};
