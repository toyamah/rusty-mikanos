use crate::graphics::PixelWriter;
use crate::{PixelColor, Vector2D};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::{Ref, RefCell, RefMut};

pub struct Window {
    actual: Rc<RefCell<ActualWindow>>,
    writer: WindowWriter,
}

struct ActualWindow {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    transparent_color: Option<PixelColor>,
}

pub struct WindowWriter {
    window: Rc<RefCell<ActualWindow>>,
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        debug_assert!(width <= i32::MAX as usize);
        debug_assert!(height <= i32::MAX as usize);

        let aw = Rc::new(RefCell::new(ActualWindow::new(width, height)));
        Self {
            actual: aw.clone(),
            writer: WindowWriter { window: aw.clone() },
        }
    }

    pub fn draw_to<W: PixelWriter + ?Sized>(&self, writer: &W, position: Vector2D<i32>) {
        match self.window().transparent_color {
            None => self.on_each_pixel(|x, y| {
                writer.write(position.x + x as i32, position.y + y as i32, &self.at(x, y))
            }),
            Some(transparent) => self.on_each_pixel(|x, y| {
                let color = self.at(x, y);
                if color != transparent {
                    writer.write(position.x + x as i32, position.y + y as i32, &color);
                }
            }),
        }
    }

    pub fn set_transparent_color(&mut self, c: PixelColor) {
        self.window_mut().transparent_color = Some(c);
    }

    pub fn width(&self) -> usize {
        self.window().width
    }

    pub fn height(&self) -> usize {
        self.window().height
    }

    pub fn writer(&self) -> &WindowWriter {
        &self.writer
    }

    fn on_each_pixel<F>(&self, f: F)
    where
        F: Fn(usize, usize) -> (),
    {
        let w = self.window();
        for y in 0..w.height {
            for x in 0..w.width {
                f(x, y);
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> PixelColor {
        self.window().data[y][x]
    }

    fn window(&self) -> Ref<'_, ActualWindow> {
        (*self.actual).borrow()
    }

    fn window_mut(&mut self) -> RefMut<'_, ActualWindow> {
        (*self.actual).borrow_mut()
    }
}

impl PixelWriter for WindowWriter {
    fn write(&self, x: i32, y: i32, color: &PixelColor) {
        (*self.window).borrow_mut().write(x, y, color)
    }

    fn width(&self) -> i32 {
        (*self.window).borrow().width as i32
    }

    fn height(&self) -> i32 {
        (*self.window).borrow().height as i32
    }
}

impl ActualWindow {
    fn new(width: usize, height: usize) -> Self {
        let data: Vec<Vec<_>> = (0..height)
            .map(|_| (0..width).map(|_| PixelColor::new(0, 0, 0)).collect())
            .collect();
        Self {
            width,
            height,
            data,
            transparent_color: None,
        }
    }

    fn write(&mut self, x: i32, y: i32, color: &PixelColor) {
        self.data[y as usize][x as usize] = *color
    }
}
