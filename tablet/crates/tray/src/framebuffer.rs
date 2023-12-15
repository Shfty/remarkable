use libremarkable::cgmath::{Point2, Vector2};
pub use libremarkable::framebuffer::common::{
    color as Color, display_temp as DisplayTemp, dither_mode as DitherMode,
    mxcfb_rect as MxcfbRect, waveform_mode as WaveformMode,
};

use crate::rect::{Position, Size, Empty};

impl Position for MxcfbRect {
    fn position(&self) -> Point2<i32> {
        Point2::new(self.left as i32, self.top as i32)
    }
}

impl Size for MxcfbRect {
    fn size(&self) -> Vector2<u32> {
        Vector2::new(self.width, self.height)
    }
}

impl Empty for MxcfbRect {
    fn empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }
}
