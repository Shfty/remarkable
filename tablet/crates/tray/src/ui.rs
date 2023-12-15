use crate::{
    framebuffer::{Color, DisplayTemp, DitherMode, MxcfbRect, WaveformMode},
    rect::{Empty, Position},
};
use gesture::{GestureCallback, GestureRecognizer};
use libremarkable::{
    cgmath::Point2,
    framebuffer::{
        core::Framebuffer, refresh::PartialRefreshMode, FramebufferDraw, FramebufferIO,
        FramebufferRefresh,
    },
};

pub struct DrawContext {
    pub fb: Framebuffer,
    pub rect: MxcfbRect,
    pub gesture_recognizer: GestureRecognizer,
}

impl Clone for DrawContext {
    fn clone(&self) -> Self {
        DrawContext {
            fb: Framebuffer::default(),
            rect: self.rect,
            gesture_recognizer: GestureRecognizer::default(),
        }
    }
}

pub trait DrawFn: Fn(DrawContext) -> DrawContext {}
impl<F> DrawFn for F where F: Fn(DrawContext) -> DrawContext {}

/// Unit widget, draws nothing and returns the provided rect
pub fn unit() -> impl DrawFn + Copy {
    move |ctx| ctx
}

/// Clear the framebuffer
pub fn clear() -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb.clear();
        ctx
    }
}

/// Refresh a region of the framebuffer
pub fn partial_refresh(
    refresh_mode: PartialRefreshMode,
    waveform_mode: WaveformMode,
    display_temp: DisplayTemp,
    dither_mode: DitherMode,
    quant_bit: i32,
    force_full_refresh: bool,
) -> impl DrawFn {
    move |ctx: DrawContext| {
        ctx.fb.partial_refresh(
            &ctx.rect,
            match &refresh_mode {
                PartialRefreshMode::DryRun => PartialRefreshMode::DryRun,
                PartialRefreshMode::Async => PartialRefreshMode::Async,
                PartialRefreshMode::Wait => PartialRefreshMode::Wait,
            },
            waveform_mode,
            display_temp,
            dither_mode,
            quant_bit,
            force_full_refresh,
        );
        ctx
    }
}

/// Trait to allow composition of DrawFn
pub trait Draw {
    fn draw(&self, ctx: DrawContext) -> DrawContext;
}

/// DrawFn calls itself in order to draw
impl<T> Draw for T
where
    T: DrawFn,
{
    fn draw(&self, ctx: DrawContext) -> DrawContext {
        self(ctx)
    }
}

/// Then executes two draws in sequence
#[derive(Copy, Clone)]
pub struct Then<A: Draw, B: Draw>(A, B);

impl<A: Draw, B: Draw> Draw for Then<A, B> {
    fn draw(&self, mut ctx: DrawContext) -> DrawContext {
        ctx = self.0.draw(ctx);
        ctx = self.1.draw(ctx);
        ctx
    }
}

/// Construction trait for Then
pub trait ThenTrait<B: Draw> {
    type Then;

    fn then(self, b: B) -> Self::Then;
}

impl<A: Draw, B: Draw> ThenTrait<B> for A {
    type Then = Then<A, B>;

    fn then(self, b: B) -> Self::Then {
        Then(self, b)
    }
}

/// Then executes two draws in sequence
#[derive(Copy, Clone)]
pub struct Overlay<A: Draw, B: Draw>(A, B);

impl<A: Draw, B: Draw> Draw for Overlay<A, B> {
    fn draw(&self, mut ctx: DrawContext) -> DrawContext {
        ctx = self.0.draw(ctx);
        let rect = ctx.rect;
        ctx = self.1.draw(ctx);
        ctx.rect = rect;
        ctx
    }
}

/// Construction trait for Then
pub trait OverlayTrait<B: Draw> {
    type Overlay;

    fn overlay(self, b: B) -> Self::Overlay;
}

impl<A: Draw, B: Draw> OverlayTrait<B> for A {
    type Overlay = Overlay<A, B>;

    fn overlay(self, b: B) -> Self::Overlay {
        Overlay(self, b)
    }
}

/// Refresh the whole framebuffer
pub fn full_refresh(
    waveform_mode: WaveformMode,
    display_temp: DisplayTemp,
    dither_mode: DitherMode,
    quant_bit: i32,
    wait_completion: bool,
) -> impl DrawFn {
    move |ctx: DrawContext| {
        ctx.fb.full_refresh(
            waveform_mode,
            display_temp,
            dither_mode,
            quant_bit,
            wait_completion,
        );
        ctx
    }
}

/// Restore a region of the framebuffer
pub fn restore_region<T: std::borrow::Borrow<[u8]>>(data: T) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb.restore_region(ctx.rect, data.borrow()).unwrap();
        ctx
    }
}

/// Dump a region of the framebuffer using a callback function
pub fn dump_region<F: Fn(Vec<u8>)>(f: F) -> impl DrawFn {
    move |ctx: DrawContext| {
        f(ctx.fb.dump_region(ctx.rect).unwrap());
        ctx
    }
}

/// Draw a filled circle
pub fn circle_stroke(rad: u32, color: Color) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb.draw_circle(ctx.rect.position(), rad, color);
        ctx
    }
}

/// Draw an unfilled circle
pub fn circle_fill(rad: u32, color: Color) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb.fill_circle(ctx.rect.position(), rad, color);
        ctx
    }
}

/// Draw a circle with distinct fill and stroke colors
pub fn circle_border(rad: u32, fill_color: Color, stroke_color: Color) -> impl Draw {
    circle_fill(rad, fill_color).then(circle_stroke(rad, stroke_color))
}

/// Draw a line of text
pub fn text(text: &str, size: f32, color: Color) -> impl DrawFn + '_ {
    move |mut ctx: DrawContext| {
        let rect = ctx.fb.draw_text(
            ctx.rect.position().cast().unwrap(),
            text,
            size,
            color,
            false,
        );
        DrawContext { rect, ..ctx }
    }
}

/// Draw a line of text aligned to the provided origin
pub fn text_aligned(
    string: &str,
    size: f32,
    origin: Point2<f32>,
    color: Color,
) -> impl DrawFn + '_ {
    move |mut ctx: DrawContext| {
        let tr = ctx.fb.draw_text(
            ctx.rect.position().cast().unwrap(),
            string,
            size,
            Default::default(),
            true,
        );

        ctx = offset_relative(Point2::new(
            -(tr.width as f32 * origin.x) as i32,
            -(tr.height as f32 * origin.y) as i32,
        ))
        .then(text(string, size, color))
        .draw(ctx);

        ctx
    }
}

/// Draw the provided RGB image, anchored at the top-left
pub fn image(image: &libremarkable::image::RgbImage) -> impl DrawFn + '_ {
    move |mut ctx: DrawContext| {
        let rect = ctx.fb.draw_image(image, ctx.rect.position());
        DrawContext { rect, ..ctx }
    }
}

/// Run the provided draw command, ignoring any resulting changes to the rect
pub fn overlay(f: impl Draw) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        let rect = ctx.rect;
        ctx = f.draw(ctx);
        ctx.rect = rect;
        ctx
    }
}

/// Offset the position of the provided draw
pub fn offset_relative(offset: Point2<i32>) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.left = (ctx.rect.left as i32 + offset.x) as u32;
        ctx.rect.top = (ctx.rect.top as i32 + offset.y) as u32;
        ctx
    }
}

/// Offset the position of the provided draw relative to the size of its containing rect
pub fn offset_absolute(offset: Point2<f32>) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx = offset_relative(Point2::new(
            (ctx.rect.width as f32 * offset.x) as i32,
            (ctx.rect.height as f32 * offset.y) as i32,
        ))(ctx);

        ctx
    }
}

/// Apply a top margin to the provided draw
pub fn margin_top(margin: i32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.top = (ctx.rect.top as i32 + margin).max(0) as u32;
        ctx.rect.height = (ctx.rect.height as i32 - margin).max(0) as u32;
        ctx
    }
}

/// Apply a left margin to the provided draw
pub fn margin_left(margin: i32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.left = (ctx.rect.left as i32 + margin).max(0) as u32;
        ctx.rect.width = (ctx.rect.width as i32 - margin).max(0) as u32;
        ctx
    }
}

/// Apply a right margin to the provided draw
pub fn margin_right(margin: i32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.width = (ctx.rect.width as i32 - margin).max(0) as u32;
        ctx
    }
}

/// Apply a top margin to the provided draw
pub fn margin_bottom(margin: i32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.height = (ctx.rect.height as i32 - margin).max(0) as u32;
        ctx
    }
}

/// Apply horizontal margins to the provided draw
pub fn margin_horizontal(margin: i32) -> impl Draw {
    margin_left(margin).then(margin_right(margin))
}

/// Apply vertical margins to the provided draw
pub fn margin_vertical(margin: i32) -> impl Draw {
    margin_top(margin).then(margin_bottom(margin))
}

/// Apply margins to all sides of the provided draw
pub fn margin(margin: i32) -> impl Draw {
    margin_horizontal(margin).then(margin_vertical(margin))
}

/// Draw a filled rectangle
pub fn rect_fill(color: Color) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb
            .fill_rect(ctx.rect.position(), ctx.rect.size(), color);
        ctx
    }
}

pub fn line(start: Point2<i32>, end: Point2<i32>, width: u32, color: Color) -> impl DrawFn + Copy {
    move |mut ctx: DrawContext| {
        ctx.rect = ctx.fb.draw_line(
            Point2::new(
                ctx.rect.left as i32 + start.x,
                ctx.rect.top as i32 + start.y,
            ),
            Point2::new(ctx.rect.left as i32 + end.x, ctx.rect.top as i32 + end.y),
            width,
            color,
        );
        ctx
    }
}

/// Draw an unfilled rectangle
pub fn rect_stroke(border_px: u32, color: Color) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.fb
            .draw_rect(ctx.rect.position(), ctx.rect.size(), border_px, color);
        ctx
    }
}

/// Draw a rectangle with distinct fill and stroke colors
pub fn rect_border(border_px: u32, fill_color: Color, stroke_color: Color) -> impl Draw {
    rect_fill(fill_color).then(rect_stroke(border_px, stroke_color))
}

/// Arrange the provided draws horizontally
pub fn horizontal<'a>(spacing: i32, draws: &'a [impl DrawFn]) -> impl DrawFn + 'a {
    move |mut ctx: DrawContext| {
        for draw in draws {
            let cached = ctx.rect;
            ctx = draw(ctx);
            let margin = ctx.rect.width as i32 + spacing;
            ctx.rect = cached;
            ctx = margin_left(margin)(ctx);
            if ctx.rect.empty() {
                break;
            }
        }
        ctx
    }
}

/// Arrange the provided draws vertically using fixed-height elements
pub fn horizontal_fixed<'a>(element_width: i32, draws: &'a [impl DrawFn]) -> impl DrawFn + 'a {
    move |mut ctx: DrawContext| {
        for draw in draws {
            ctx = overlay(draw)(ctx);
            ctx = margin_left(element_width)(ctx);
            if ctx.rect.empty() {
                break;
            }
        }
        ctx
    }
}

/// Arrange the provided draws vertically
pub fn vertical<'a>(spacing: i32, draws: &'a [impl DrawFn]) -> impl DrawFn + 'a {
    move |mut ctx: DrawContext| {
        for draw in draws {
            let cached = ctx.rect;
            ctx = draw(ctx);
            let margin = ctx.rect.height as i32 + spacing;
            ctx.rect = cached;
            ctx = margin_top(margin)(ctx);
            if ctx.rect.empty() {
                break;
            }
        }
        ctx
    }
}

/// Arrange the provided draws vertically using fixed-height elements
pub fn vertical_fixed<'a>(element_height: i32, draws: &'a [impl DrawFn]) -> impl DrawFn + 'a {
    move |mut ctx: DrawContext| {
        for draw in draws {
            ctx = overlay(draw)(ctx);
            ctx = margin_top(element_height)(ctx);
            if ctx.rect.empty() {
                break;
            }
        }
        ctx
    }
}

/// Injects a gesture recognizer for the current rect
pub fn recognize_gesture(g: impl GestureCallback + Clone + Send + Sync + 'static) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.gesture_recognizer =
            ctx.gesture_recognizer
                .with_callback(gesture::recognize_starting_zone(
                    ctx.rect.position().cast().unwrap(),
                    ctx.rect.size().cast().unwrap(),
                    g.clone(),
                ));
        ctx
    }
}

/// Override the current rect x
pub fn set_x(x: u32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.left = x;
        ctx
    }
}

/// Override the current rect y
pub fn set_y(y: u32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.top = y;
        ctx
    }
}

/// Override the current rect position
pub fn set_position(x: u32, y: u32) -> impl Draw {
    set_x(x).then(set_y(y))
}

/// Override the current rect width
pub fn set_width(width: u32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.width = width;
        ctx
    }
}

/// Override the current rect height
pub fn set_height(height: u32) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect.height = height;
        ctx
    }
}

/// Override the current rect size
pub fn set_size(width: u32, height: u32) -> impl Draw {
    set_width(width).then(set_height(height))
}

/// Override the current rect
pub fn set_rect(rect: MxcfbRect) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        ctx.rect = rect;
        ctx
    }
}
