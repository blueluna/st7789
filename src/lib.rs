#![no_std]

//! This crate provides a ST7789 driver to connect to TFT displays.

pub mod instruction;

use crate::instruction::Instruction;
use num_derive::ToPrimitive;
use num_traits::ToPrimitive;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;

/// ST7789 driver to connect to TFT displays.
pub struct ST7789<SPI, DC, RST>
where
    SPI: spi::Write<u8>,
    DC: OutputPin,
    RST: OutputPin,
{
    /// SPI
    spi: SPI,

    /// Data/command pin.
    dc: DC,

    /// Reset pin.
    rst: RST,

    /// Whether the display is RGB (true) or BGR (false)
    rgb: bool,

    /// Whether the colours are inverted (true) or not (false)
    inverted: bool,

    /// Screen size
    size_x: u16,
    size_y: u16,

    /// Global image offset
    dx: u16,
    dy: u16,
}

/// Display orientation.
#[derive(ToPrimitive)]
pub enum Orientation {
    Portrait = 0x00,
    Landscape = 0x60,
    PortraitSwapped = 0xC0,
    LandscapeSwapped = 0xA0,
}

impl<SPI, DC, RST> ST7789<SPI, DC, RST>
where
    SPI: spi::Write<u8>,
    DC: OutputPin,
    RST: OutputPin,
{
    ///
    /// Creates a new ST7789 driver instance
    ///
    /// # Arguments
    ///
    /// * `spi` - an SPI interface to use for talking to the display
    /// * `dc` - data/clock pin switch
    /// * `rst` - display hard reset pin
    /// * `size_x` - x axis resolution of the display in pixels
    /// * `size_y` - y axis resolution of the display in pixels
    ///
    pub fn new(spi: SPI, dc: DC, rst: RST, size_x: u16, size_y: u16) -> Self {
        let display = ST7789 {
            spi,
            dc,
            rst,
            rgb: true,
            inverted: true,
            dx: 0,
            dy: 0,
            size_x: size_x,
            size_y: size_y,
        };

        display
    }

    ///
    /// Runs commands to initialize the display
    ///
    /// # Arguments
    ///
    /// * `delay` - a delay provided for the MCU/MPU this is running on
    pub fn init<DELAY>(&mut self, delay: &mut DELAY) -> Result<(), ()>
    where
        DELAY: DelayMs<u8>,
    {
        self.hard_reset()?;
        self.write_command(Instruction::SWRESET, None)?;
        delay.delay_ms(200);
        self.write_command(Instruction::SLPOUT, None)?;
        delay.delay_ms(200);
        self.write_command(Instruction::FRMCTR1, Some(&[0x01, 0x2C, 0x2D]))?;
        self.write_command(Instruction::FRMCTR2, Some(&[0x01, 0x2C, 0x2D]))?;
        self.write_command(
            Instruction::FRMCTR3,
            Some(&[0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D]),
        )?;
        self.write_command(Instruction::INVCTR, Some(&[0x07]))?;
        self.write_command(Instruction::PWCTR1, Some(&[0xA2, 0x02, 0x84]))?;
        self.write_command(Instruction::PWCTR2, Some(&[0xC5]))?;
        self.write_command(Instruction::PWCTR3, Some(&[0x0A, 0x00]))?;
        self.write_command(Instruction::PWCTR4, Some(&[0x8A, 0x2A]))?;
        self.write_command(Instruction::PWCTR5, Some(&[0x8A, 0xEE]))?;
        self.write_command(Instruction::VMCTR1, Some(&[0x0E]))?;
        if self.inverted {
            self.write_command(Instruction::INVON, None)?;
        } else {
            self.write_command(Instruction::INVOFF, None)?;
        }
        if self.rgb {
            self.write_command(Instruction::MADCTL, Some(&[0x00]))?;
        } else {
            self.write_command(Instruction::MADCTL, Some(&[0x08]))?;
        }
        self.write_command(Instruction::COLMOD, Some(&[0x05]))?;
        self.write_command(Instruction::DISPON, None)?;
        delay.delay_ms(200);
        Ok(())
    }

    pub fn hard_reset(&mut self) -> Result<(), ()> {
        self.rst.set_high().map_err(|_| ())?;
        self.rst.set_low().map_err(|_| ())?;
        self.rst.set_high().map_err(|_| ())
    }

    fn write_command(&mut self, command: Instruction, params: Option<&[u8]>) -> Result<(), ()> {
        self.dc.set_low().map_err(|_| ())?;
        self.spi
            .write(&[command.to_u8().unwrap()])
            .map_err(|_| ())?;
        if params.is_some() {
            self.start_data()?;
            self.write_data(params.unwrap())?;
        }
        Ok(())
    }

    fn start_data(&mut self) -> Result<(), ()> {
        self.dc.set_high().map_err(|_| ())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), ()> {
        self.spi.write(data).map_err(|_| ())
    }

    /// Writes a data word to the display.
    fn write_word(&mut self, value: u16) -> Result<(), ()> {
        self.write_data(&value.to_be_bytes())
    }

    pub fn set_orientation(&mut self, orientation: &Orientation) -> Result<(), ()> {
        if self.rgb {
            self.write_command(Instruction::MADCTL, Some(&[orientation.to_u8().unwrap()]))?;
        } else {
            self.write_command(
                Instruction::MADCTL,
                Some(&[orientation.to_u8().unwrap() | 0x08]),
            )?;
        }
        Ok(())
    }

    /// Sets the global offset of the displayed image
    pub fn set_offset(&mut self, dx: u16, dy: u16) {
        self.dx = dx;
        self.dy = dy;
    }

    /// Sets the address window for the display.
    fn set_address_window(&mut self, sx: u16, sy: u16, ex: u16, ey: u16) -> Result<(), ()> {
        self.write_command(Instruction::CASET, None)?;
        self.start_data()?;
        self.write_word(sx + self.dx)?;
        self.write_word(ex + self.dx)?;
        self.write_command(Instruction::RASET, None)?;
        self.start_data()?;
        self.write_word(sy + self.dy)?;
        self.write_word(ey + self.dy)
    }

    /// Sets a pixel color at the given coords.
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), ()> {
        self.set_address_window(x, y, x, y)?;
        self.write_command(Instruction::RAMWR, None)?;
        self.start_data()?;
        self.write_word(color)
    }
}

#[cfg(feature = "graphics")]
extern crate embedded_graphics;
#[cfg(feature = "graphics")]
use self::embedded_graphics::{
    drawable::Pixel, pixelcolor::raw::*, pixelcolor::Rgb565, prelude::Size, DrawTarget,
};

#[cfg(feature = "graphics")]
impl<SPI, DC, RST> DrawTarget<Rgb565> for ST7789<SPI, DC, RST>
where
    SPI: spi::Write<u8>,
    DC: OutputPin,
    RST: OutputPin,
{
    type Error = SPI::Error;

    fn draw_pixel(&mut self, pixel: Pixel<Rgb565>) -> Result<(), Self::Error> {
        let color = RawU16::from(pixel.1).into_inner();
        let x = pixel.0.x as u16;
        let y = pixel.0.y as u16;

        self.set_pixel(x, y, color).expect("pixel write failed");

        Ok(())
    }

    fn size(&self) -> Size {
        Size::new(self.size_x.into(), self.size_y.into())
    }
}
