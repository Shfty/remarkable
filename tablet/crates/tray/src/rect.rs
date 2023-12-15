use libremarkable::cgmath::{Point2, Vector2};

pub trait Position {
    fn position(&self) -> Point2<i32>;
}

pub trait Size {
    fn size(&self) -> Vector2<u32>;
}

pub trait Empty {
    fn empty(&self) -> bool;
}

pub trait Rect: Position + Size + Empty {}
impl<T> Rect for T where T: Position + Size + Empty {}
