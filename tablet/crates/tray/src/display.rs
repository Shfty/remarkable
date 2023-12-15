pub use libremarkable::dimensions::{
    DISPLAYHEIGHT as DISPLAY_HEIGHT, DISPLAYWIDTH as DISPLAY_WIDTH,
};

use libremarkable::framebuffer::common::mxcfb_rect as MxcfbRect;

pub const DISPLAY_RECT: MxcfbRect = MxcfbRect {
    top: 0,
    left: 0,
    width: DISPLAY_WIDTH as u32,
    height: DISPLAY_HEIGHT as u32,
};
