#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_graphics::{
  fonts::{Font6x8, Text},
  mock_display::MockDisplay,
  pixelcolor::BinaryColor,
  prelude::*,
  primitives::Circle,
  style::{PrimitiveStyle, TextStyle},
};
use panic_rtt_target as _;
use pcd8544::PCD8544;
use stm32f1xx_hal::{
  pac,
  prelude::*,
  spi::{Mode, NoMiso, Phase, Polarity, Spi},
};

#[entry]
fn main() -> ! {
  rtt_target::rtt_init_print!();

  rtt_target::rprintln!("here");

  let mut cp: cortex_m::Peripherals = cortex_m::Peripherals::take().unwrap();
  let mut peripherals = pac::Peripherals::take().unwrap();
  let mut rcc = peripherals.RCC.constrain();

  let mut flash = peripherals.FLASH.constrain();
  let clocks = rcc.cfgr.freeze(&mut flash.acr);

  let mut gpioa = peripherals.GPIOA.split(&mut rcc.apb2);

  let pins = (
    gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl),
    NoMiso,
    gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl),
  );

  let spi_mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
  };

  let mut afio = peripherals.AFIO.constrain(&mut rcc.apb2);

  let spi = Spi::spi1(
    peripherals.SPI1,
    pins,
    &mut afio.mapr,
    spi_mode,
    1.hz(),
    clocks,
    &mut rcc.apb2,
  );

  // let pcd_clk = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);
  // let pcd_din = gpioa.pa7.into_push_pull_output(&mut gpioa.crl);
  let pcd_dc = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
  let pcd_ce = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
  let pcd_rst = gpioa.pa3.into_push_pull_output(&mut gpioa.crl);
  let pcd_light = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

  // let mut display = PCD8544::new(pcd_clk, pcd_din, pcd_dc, pcd_ce, pcd_rst, pcd_light)
  //   .expect("Infallible cannot fail");

  let mut display =
    PCD8544::new(spi, pcd_dc, pcd_ce, pcd_rst, pcd_light).expect("Infallible cannot fail");

  display.init().unwrap();
  display.reset().expect("Infallible cannot fail");

  let c =
    Circle::new(Point::new(20, 20), 8).into_styled(PrimitiveStyle::with_fill(BinaryColor::On));
  let t = Text::new("Hello Rust! :)", Point::new(0, 16))
    .into_styled(TextStyle::new(Font6x8, BinaryColor::On));

  c.draw(&mut display).unwrap();
  t.draw(&mut display).unwrap();

  display.flush().unwrap();

  loop {}
}
