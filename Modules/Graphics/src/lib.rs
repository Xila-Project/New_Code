#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

mod Display;
mod Input;
//mod Window;

pub use Display::*;
pub use Input::*;
//pub use Window::*;

pub use lvgl;
pub use lvgl::sys;

#[repr(transparent)]
pub struct Draw_buffer_type<const Buffer_size: usize>(lvgl::DrawBuffer<Buffer_size>);

impl<const Buffer_size: usize> Default for Draw_buffer_type<Buffer_size> {
    fn default() -> Self {
        Draw_buffer_type(lvgl::DrawBuffer::<Buffer_size>::default())
    }
}

impl<const Buffer_size: usize> From<Draw_buffer_type<Buffer_size>>
    for lvgl::DrawBuffer<Buffer_size>
{
    fn from(Draw_buffer: Draw_buffer_type<Buffer_size>) -> Self {
        Draw_buffer.0
    }
}
