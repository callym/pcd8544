extern crate embedded_hal;
extern crate pcd8544;

use embedded_hal::digital::v2::OutputPin;
use pcd8544::PCD8544;
use std::fmt::Write as _;

#[derive(Debug)]
pub struct DummyOutputPin {}

impl DummyOutputPin {
    pub fn new() -> Self {
        DummyOutputPin {}
    }
}

impl OutputPin for DummyOutputPin {
    type Error = ();
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() -> () {
    let pcd_light = DummyOutputPin::new();
    let pcd_clk = DummyOutputPin::new();
    let pcd_din = DummyOutputPin::new();
    let pcd_dc = DummyOutputPin::new();
    let pcd_ce = DummyOutputPin::new();
    let pcd_rst = DummyOutputPin::new();

    let mut display = PCD8544::new(pcd_clk, pcd_din, pcd_dc, pcd_ce, pcd_rst, pcd_light).unwrap();

    display.reset().unwrap();
    writeln!(display, "Hello World").unwrap();
}
