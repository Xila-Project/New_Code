use Graphics::{
    Color_ARGB8888_type, Point_type, Pointer_data_type, Screen_read_data_type,
    Screen_write_data_type, Touch_type,
};

use sdl2::{
    event, mouse, pixels,
    render::Canvas,
    video::{self},
    EventPump,
};
use File_system::{Device_trait, Result_type};

use std::{marker::PhantomData, mem::size_of, process::exit, sync::RwLock};

pub struct Screen_device_type<const Buffer_size: usize>(
    RwLock<Canvas<video::Window>>,
    PhantomData<[u8; Buffer_size]>,
);

unsafe impl<const Buffer_size: usize> Send for Screen_device_type<Buffer_size> {}

unsafe impl<const Buffer_size: usize> Sync for Screen_device_type<Buffer_size> {}

impl<const Buffer_size: usize> Screen_device_type<Buffer_size> {
    pub fn New(Window: video::Window) -> Result<Self, String> {
        let mut Canvas = Window
            .into_canvas()
            .build()
            .map_err(|Error| format!("Error building canvas: {:?}", Error))?;

        Canvas.clear();
        Canvas.present();

        Ok(Self(RwLock::new(Canvas), PhantomData))
    }

    fn Get_resolution(&self) -> Result<Point_type, String> {
        self.0
            .read()
            .map_err(|Error| format!("Poisoned RwLock: {:?}", Error))?
            .output_size()
            .map(|(Width, Height)| Point_type::New(Width as i16, Height as i16))
            .map_err(|Error| format!("Error getting resolution: {:?}", Error))
    }

    fn Update(&self, Data: &Screen_write_data_type<Buffer_size>) -> Result<(), String> {
        let mut Buffer_iterator = Data.Get_buffer().iter();

        let Point_1 = Data.Get_area().Get_point_1();
        let Point_2 = Data.Get_area().Get_point_2();

        let mut Canvas = self
            .0
            .write()
            .map_err(|Error| format!("Poisoned RwLock: {:?}", Error))?;

        for Y in Point_1.Get_y() as i32..=Point_2.Get_y() as i32 {
            for X in Point_1.Get_x() as i32..=Point_2.Get_x() as i32 {
                let Color = Buffer_iterator
                    .next()
                    .ok_or("Buffer is too short.".to_string())?;

                let Color: Color_ARGB8888_type = (*Color).into();

                Canvas.set_draw_color(pixels::Color::RGB(
                    Color.Get_red(),
                    Color.Get_green(),
                    Color.Get_blue(),
                ));

                let _ = Canvas.draw_point(sdl2::rect::Point::new(X, Y));
            }
        }
        Canvas.present();

        Ok(())
    }
}

impl<const Buffer_size: usize> Device_trait for Screen_device_type<Buffer_size> {
    fn Read(&self, Buffer: &mut [u8]) -> File_system::Result_type<usize> {
        let Data: &mut Screen_read_data_type = Buffer
            .try_into()
            .map_err(|_| File_system::Error_type::Invalid_input)?;

        Data.Set_resolution(
            self.Get_resolution()
                .map_err(|_| File_system::Error_type::Internal_error)?,
        );

        Ok(size_of::<Screen_read_data_type>())
    }

    fn Write(&self, Buffer: &[u8]) -> File_system::Result_type<usize> {
        let Data: &Screen_write_data_type<Buffer_size> = Buffer
            .try_into()
            .map_err(|_| File_system::Error_type::Invalid_input)?;

        self.Update(Data).expect("Error updating screen.");

        Ok(Buffer_size)
    }

    fn Get_size(&self) -> File_system::Result_type<usize> {
        Ok(size_of::<Self>())
    }

    fn Set_position(&self, _: &File_system::Position_type) -> File_system::Result_type<usize> {
        Err(File_system::Error_type::Unsupported_operation)
    }

    fn Flush(&self) -> File_system::Result_type<()> {
        Ok(())
    }
}

pub struct Pointer_device_type {
    Window_identifier: u32,
    Event_pump: RwLock<EventPump>,
    Last_input: RwLock<Pointer_data_type>,
}

unsafe impl Send for Pointer_device_type {}

unsafe impl Sync for Pointer_device_type {}

impl Pointer_device_type {
    pub fn New(Window_identifier: u32, Event_pump: EventPump) -> Self {
        Self {
            Window_identifier,
            Event_pump: RwLock::new(Event_pump),
            Last_input: RwLock::new(Pointer_data_type::New(
                Point_type::New(0, 0),
                Touch_type::Released,
            )),
        }
    }

    pub fn Update(&self) -> Result_type<()> {
        let mut Last_input = self.Last_input.write()?;

        for Event in self.Event_pump.write()?.poll_iter() {
            match Event {
                event::Event::Quit { .. } => exit(0),
                event::Event::MouseButtonDown {
                    timestamp: _,
                    window_id,
                    which: _,
                    mouse_btn,
                    clicks: _,
                    x,
                    y,
                } => {
                    if (window_id == self.Window_identifier)
                        && (mouse_btn == mouse::MouseButton::Left)
                    {
                        Last_input.Set(Point_type::New(x as i16, y as i16), Touch_type::Pressed);
                    }
                }
                event::Event::MouseButtonUp {
                    timestamp: _,
                    window_id,
                    which: _,
                    mouse_btn,
                    clicks: _,
                    ..
                } => {
                    if (window_id == self.Window_identifier)
                        && (mouse_btn == mouse::MouseButton::Left)
                    {
                        Last_input.Set_touch(Touch_type::Released);
                    }
                }
                event::Event::MouseMotion {
                    timestamp: _,
                    window_id,
                    which: _,
                    mousestate,
                    x,
                    y,
                    ..
                } => {
                    if (window_id == self.Window_identifier) && (mousestate.left()) {
                        Last_input.Set_point(Point_type::New(x as i16, y as i16));
                    }
                }
                _ => {}
            };
        }

        Ok(())
    }
}

impl Device_trait for Pointer_device_type {
    fn Read(&self, Buffer: &mut [u8]) -> File_system::Result_type<usize> {
        if self.Update().is_err() {
            return Err(File_system::Error_type::Internal_error);
        }

        let Input: &mut Pointer_data_type = Buffer
            .try_into()
            .map_err(|_| File_system::Error_type::Invalid_input)?;

        *Input = *self.Last_input.read()?;

        Ok(size_of::<Pointer_data_type>())
    }

    fn Write(&self, _: &[u8]) -> File_system::Result_type<usize> {
        Err(File_system::Error_type::Unsupported_operation)
    }

    fn Get_size(&self) -> File_system::Result_type<usize> {
        Ok(size_of::<Pointer_data_type>())
    }

    fn Set_position(&self, _: &File_system::Position_type) -> File_system::Result_type<usize> {
        Err(File_system::Error_type::Unsupported_operation)
    }

    fn Flush(&self) -> File_system::Result_type<()> {
        Ok(())
    }
}

pub fn New_touchscreen<const Buffer_size: usize>(
    Size: Point_type,
) -> Result<(Screen_device_type<Buffer_size>, Pointer_device_type), String> {
    let Context = sdl2::init().map_err(|Error| format!("Error initializing SDL2: {:?}", Error))?;

    let Video_subsystem = Context
        .video()
        .map_err(|Error| format!("Error getting video subsystem: {:?}", Error))?;

    let Window = Video_subsystem
        .window("Xila", Size.Get_x() as u32, Size.Get_y() as u32)
        .position_centered()
        .build()
        .map_err(|Error| format!("Error building window: {:?}", Error))?;

    let Event_pump = Context
        .event_pump()
        .map_err(|Error| format!("Error getting event pump: {:?}", Error))?;

    let Pointer = Pointer_device_type::New(Window.id(), Event_pump);

    let Screen = Screen_device_type::New(Window)?;

    Ok((Screen, Pointer))
}
