use anyhow::Result;
use embassy_time::Timer;
use embedded_graphics_core::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
    Pixel,
};
use log::info;
use std::borrow::Borrow;

use esp_idf_svc::hal::{
    gpio::{AnyOutputPin, Output, Pin, PinDriver},
    prelude::*,
    spi::{config::LineWidth, *},
    task::*,
};

pub struct Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    qspi: SpiDeviceDriver<'d, T>,
    reset: PinDriver<'d, AnyOutputPin, Output>,
}

macro_rules! command_ops {
    ($cmd: expr$(, $itms:expr)*) => {
        {
        super let header = 0x0200_u16.to_be_bytes();
        super let cmd = (($cmd as u16) << 8).to_be_bytes();
        [
            Operation::WriteWithWidth(&header, ::esp_idf_svc::hal::spi::config::LineWidth::Single),
            Operation::WriteWithWidth(&cmd, ::esp_idf_svc::hal::spi::config::LineWidth::Single),
            $($itms),*
        ]
        }
    };
}

macro_rules! write_buf {
    ($buf: expr) => {{
        super let buf = $buf;

        Operation::WriteWithWidth(&buf, ::esp_idf_svc::hal::spi::config::LineWidth::Single)
    }};
}

impl<'d, T> Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    pub fn new(qspi: SpiDeviceDriver<'d, T>, reset: AnyOutputPin) -> Result<Self> {
        Ok(Self {
            qspi,
            reset: PinDriver::output(reset)?,
        })
    }

    pub async fn init(&mut self) -> Result<()> {
        self.reset.set_high()?;

        Timer::after_millis(1).await;

        self.reset.set_low()?;

        Timer::after_millis(20).await;

        self.reset.set_high()?;

        Timer::after_millis(50).await;

        info!("AMOLED RESET complete");

        self.manufacturer_init().await?;

        info!("AMOLED manufacturer init complete");

        Ok(())
    }

    async fn manufacturer_init(&mut self) -> Result<()> {
        // Set page
        let mut ops = command_ops![0xFE, write_buf!([0x20])];
        self.qspi.transaction(&mut ops)?;

        // MIPI off
        let mut ops = command_ops![0x26, write_buf!([0x0A])];
        self.qspi.transaction(&mut ops)?;

        // SPI write RAM
        let mut ops = command_ops![0x24, write_buf!([0x80])];
        self.qspi.transaction(&mut ops)?;

        // Set page
        let mut ops = command_ops![0xFE, write_buf!([0x00])];
        self.qspi.transaction(&mut ops)?;

        // Pixel format (8 bit color)
        let mut ops = command_ops![0x3A, write_buf!([0x77])];
        self.qspi.transaction(&mut ops)?;

        // Display mode (internal timing)
        let mut ops = command_ops![0xC2, write_buf!([0x00])];
        self.qspi.transaction(&mut ops)?;

        Timer::after_millis(10).await;

        // Skip: Tearing Effect since not hooked up and considered optional
        // self.qspi
        //     .transaction(&mut command_ops![0x35, write_buf!([0x00])])?;

        // Display brightness 0
        let mut ops = command_ops![0x51, write_buf!([0x00])];
        self.qspi.transaction(&mut ops)?;

        // Leave sleep
        let mut ops = command_ops![0x11];
        self.qspi.transaction(&mut ops)?;

        Timer::after_millis(120).await;

        // Display on
        let mut ops = command_ops![0x29];
        self.qspi.transaction(&mut ops)?;

        Timer::after_millis(10).await;

        // Max brightness
        let mut ops = command_ops![0x51, write_buf!([0xFF])];
        self.qspi.transaction(&mut ops)?;

        Ok(())
    }

    pub fn all_pixels(&mut self, on: bool) -> Result<()> {
        self.qspi
            .transaction(&mut command_ops![if on { 0x23 } else { 0x22 }])?;

        Ok(())
    }
}

impl<'d, T> Dimensions for Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    fn bounding_box(&self) -> embedded_graphics_core::primitives::Rectangle {
        Rectangle::new(Point::zero(), Size::new(600, 450))
    }
}

impl<'d, T> DrawTarget for Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    type Color = Rgb888;
    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels
            .into_iter()
            .filter(|Pixel(p, _)| self.bounding_box().contains(*p))
        {
            todo!()
        }

        Ok(())
    }
}
