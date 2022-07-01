use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

lazy_static! {
    static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        // SAFETY: 0xb8000 is the starting address of VGA buffer.
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq)]
#[repr(u8)]
enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    byte: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII byte or new line.
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // Print blocks for non printable chars.
                _ => self.write_byte(0xfe),
            }
        }
        Ok(())
    }
}

impl Writer {
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                self.write(
                    /* row= */ BUFFER_HEIGHT - 1,
                    self.column_position,
                    ScreenChar {
                        byte,
                        color_code: self.color_code,
                    },
                );
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.write(row - 1, col, self.read(row, col));
            }
        }
        for col in 0..BUFFER_WIDTH {
            self.write(
                BUFFER_HEIGHT - 1,
                col,
                ScreenChar {
                    byte: b' ',
                    color_code: self.color_code,
                },
            );
        }
        self.column_position = 0;
    }

    fn write(&mut self, row: usize, col: usize, c: ScreenChar) {
        // SAFETY: we are writing to a valid VGA buffer location.
        unsafe {
            core::ptr::write_volatile(&mut self.buffer.chars[row][col] as *mut ScreenChar, c);
        }
    }

    fn read(&self, row: usize, col: usize) -> ScreenChar {
        // SAFETY: we are readding from a valid VGA buffer location.
        unsafe { core::ptr::read_volatile(&self.buffer.chars[row][col] as *const ScreenChar) }
    }
}

#[test_case]
fn println_single_line() {
    let s = "single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        assert_eq!(WRITER.lock().read(BUFFER_HEIGHT - 2, i).byte, c as u8);
    }
}

#[test_case]
fn println_multi_line() {
    let f = "first line";
    let s = "second line";
    println!("{}", f);
    println!("{}", s);
    for (i, c) in f.chars().enumerate() {
        assert_eq!(WRITER.lock().read(BUFFER_HEIGHT - 3, i).byte, c as u8);
    }
    for (i, c) in s.chars().enumerate() {
        assert_eq!(WRITER.lock().read(BUFFER_HEIGHT - 2, i).byte, c as u8);
    }
}
