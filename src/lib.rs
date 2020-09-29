#![no_std]
#![forbid(deprecated)]
#![deny(warnings)]

use core::convert::{Infallible, TryInto};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use embedded_hal::{blocking, digital::v2::OutputPin};

const WIDTH: u8 = 84;
const HEIGHT: u8 = 48;
const ROWS: u8 = HEIGHT / 8;

const MAX_X: u32 = WIDTH as u32 - 1;
const MAX_Y: u32 = HEIGHT as u32 - 1;
const HEIGHT_BANKS: usize = 6;

#[repr(u8)]
pub enum TemperatureCoefficient {
  TC0 = 0,
  TC1 = 1,
  TC2 = 2,
  TC3 = 3,
}

#[repr(u8)]
pub enum BiasMode {
  Bias1To100 = 0,
  Bias1To80 = 1,
  Bias1To65 = 2,
  Bias1To48 = 3,
  Bias1To40 = 4,
  Bias1To24 = 5,
  Bias1To18 = 6,
  Bias1To10 = 7,
}

#[repr(u8)]
pub enum DisplayMode {
  DisplayBlank = 0b000,
  NormalMode = 0b100,
  AllSegmentsOn = 0b001,
  InverseVideoMode = 0b101,
}

#[derive(Debug)]
pub enum OutputError<SPIE, DCE, CEE, RSTE, LIGHTE> {
  SPIError(SPIE),
  DCError(DCE),
  CEError(CEE),
  RSTError(RSTE),
  LIGHTError(LIGHTE),
}

pub struct PCD8544<SPI, DC, CE, RST, LIGHT>
where
  SPI: blocking::spi::Write<u8>,
  DC: OutputPin,
  CE: OutputPin,
  RST: OutputPin,
  LIGHT: OutputPin,
{
  spi: SPI,
  dc: DC,
  ce: CE,
  rst: RST,
  light: LIGHT,
  power_down_control: bool,
  entry_mode: bool,
  extended_instruction_set: bool,
  framebuffer: [[u8; WIDTH as usize]; HEIGHT_BANKS],
}

impl<SPI, DC, CE, RST, LIGHT> PCD8544<SPI, DC, CE, RST, LIGHT>
where
  SPI: blocking::spi::Write<u8>,
  DC: OutputPin,
  CE: OutputPin,
  RST: OutputPin,
  LIGHT: OutputPin,
{
  // TODO: somehow add type alias for this, like:
  //   type Error = OutputError<SPI::Error, DC::Error, ...>;
  // This is not yet possible, see
  // https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md

  pub fn new(
    spi: SPI,
    dc: DC,
    mut ce: CE,
    mut rst: RST,
    light: LIGHT,
  ) -> Result<
    PCD8544<SPI, DC, CE, RST, LIGHT>,
    OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>,
  > {
    assert_eq!(ROWS as usize, HEIGHT_BANKS);
    rst.set_low().map_err(|e| OutputError::RSTError(e))?;
    ce.set_high().map_err(|e| OutputError::CEError(e))?;
    Ok(PCD8544 {
      spi,
      dc,
      ce,
      rst,
      light,
      power_down_control: false,
      entry_mode: false,
      extended_instruction_set: false,
      framebuffer: [[0; WIDTH as usize]; HEIGHT_BANKS],
    })
  }

  pub fn reset(
    &mut self,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.rst.set_low().map_err(|e| OutputError::RSTError(e))?;
    self.init()?;
    Ok(())
  }

  pub fn init(
    &mut self,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    // reset the display
    self.rst.set_low().map_err(|e| OutputError::RSTError(e))?;
    self.rst.set_high().map_err(|e| OutputError::RSTError(e))?;

    // reset state variables
    self.power_down_control = false;
    self.entry_mode = false;
    self.extended_instruction_set = false;

    // write init configuration
    self.enable_extended_commands(true)?;
    self.set_contrast(56_u8)?;
    self.set_temperature_coefficient(TemperatureCoefficient::TC3)?;
    self.set_bias_mode(BiasMode::Bias1To40)?;
    self.enable_extended_commands(false)?;
    self.set_display_mode(DisplayMode::NormalMode)?;

    // clear display data
    self.clear()?;
    Ok(())
  }

  pub fn clear(
    &mut self,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    <Self as DrawTarget<BinaryColor>>::clear(self, BinaryColor::Off).unwrap();

    Ok(())
  }

  pub fn set_power_down(
    &mut self,
    power_down: bool,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.power_down_control = power_down;
    self.write_current_function_set()?;
    Ok(())
  }

  pub fn set_entry_mode(
    &mut self,
    entry_mode: bool,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.entry_mode = entry_mode;
    self.write_current_function_set()?;
    Ok(())
  }

  pub fn set_light(
    &mut self,
    enabled: bool,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    if enabled {
      self
        .light
        .set_low()
        .map_err(|e| OutputError::LIGHTError(e))?;
    } else {
      self
        .light
        .set_high()
        .map_err(|e| OutputError::LIGHTError(e))?;
    }
    Ok(())
  }

  pub fn set_display_mode(
    &mut self,
    mode: DisplayMode,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_command(0x08 | mode as u8)
  }

  pub fn set_bias_mode(
    &mut self,
    bias: BiasMode,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_command(0x10 | bias as u8)
  }

  pub fn set_temperature_coefficient(
    &mut self,
    coefficient: TemperatureCoefficient,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_command(0x04 | coefficient as u8)
  }

  /// contrast in range of 0..128
  pub fn set_contrast(
    &mut self,
    contrast: u8,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_command(0x80 | contrast)
  }

  pub fn enable_extended_commands(
    &mut self,
    enable: bool,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.extended_instruction_set = enable;
    self.write_current_function_set()?;
    Ok(())
  }

  fn write_current_function_set(
    &mut self,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    let power = self.power_down_control;
    let entry = self.entry_mode;
    let extended = self.extended_instruction_set;
    self.write_function_set(power, entry, extended)?;
    Ok(())
  }

  fn write_function_set(
    &mut self,
    power_down_control: bool,
    entry_mode: bool,
    extended_instruction_set: bool,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    let mut val = 0x20;
    if power_down_control {
      val |= 0x04;
    }
    if entry_mode {
      val |= 0x02;
    }
    if extended_instruction_set {
      val |= 0x01;
    }
    self.write_command(val)?;
    Ok(())
  }

  pub fn write_command(
    &mut self,
    value: u8,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_byte(false, value)?;
    Ok(())
  }

  pub fn write_data(
    &mut self,
    value: u8,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    self.write_byte(true, value)?;
    Ok(())
  }

  fn write_byte(
    &mut self,
    data: bool,
    value: u8,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    if data {
      self.dc.set_high().map_err(|e| OutputError::DCError(e))?;
    } else {
      self.dc.set_low().map_err(|e| OutputError::DCError(e))?;
    }
    self.ce.set_low().map_err(|e| OutputError::CEError(e))?;

    self
      .spi
      .write(&[value])
      .map_err(|e| OutputError::SPIError(e))?;

    self.ce.set_high().map_err(|e| OutputError::CEError(e))?;
    Ok(())
  }

  /// Transfers internal framebuffer data to PCD8544.
  pub fn flush(
    &mut self,
  ) -> Result<(), OutputError<SPI::Error, DC::Error, CE::Error, RST::Error, LIGHT::Error>> {
    for row in self.framebuffer.clone().iter() {
      for byte in row.iter() {
        self.write_data(*byte)?;
      }
    }

    Ok(())
  }
}

impl<SPI, DC, CE, RST, LIGHT> DrawTarget<BinaryColor> for PCD8544<SPI, DC, CE, RST, LIGHT>
where
  SPI: blocking::spi::Write<u8>,
  DC: OutputPin,
  CE: OutputPin,
  RST: OutputPin,
  LIGHT: OutputPin,
{
  type Error = Infallible;

  fn draw_pixel(&mut self, pixel: Pixel<BinaryColor>) -> Result<(), Infallible> {
    let Pixel(coord, color) = pixel;
    if let Ok((x @ 0..=MAX_X, y @ 0..=MAX_Y)) = coord.try_into() {
      let byte: &mut u8 = &mut self.framebuffer[(y / 8) as usize][x as usize];
      let mask: u8 = 1 << (y % 8);
      match color {
        BinaryColor::On => *byte |= mask,
        BinaryColor::Off => *byte &= !mask,
      };
    }
    Ok(())
  }

  fn size(&self) -> Size {
    Size::new(WIDTH as u32, HEIGHT as u32)
  }

  fn clear(&mut self, color: BinaryColor) -> Result<(), Infallible> {
    let byte: u8 = match color {
      BinaryColor::On => 0xff,
      BinaryColor::Off => 0x00,
    };
    for row in self.framebuffer.iter_mut() {
      for pos in row.iter_mut() {
        *pos = byte;
      }
    }
    Ok(())
  }
}
