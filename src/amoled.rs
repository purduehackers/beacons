use anyhow::Result;
use embassy_time::Timer;
use std::borrow::Borrow;

use esp_idf_svc::hal::{
    gpio::{AnyOutputPin, Output, Pin, PinDriver},
    prelude::*,
    spi::*,
    task::*,
};

pub struct Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    qspi: SpiDeviceDriver<'d, T>,
    reset: PinDriver<'d, AnyOutputPin, Output>,
}

impl<'d, T> Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    pub async fn new(qspi: SpiDeviceDriver<'d, T>, reset: AnyOutputPin) -> Result<Self> {
        let mut s = Self {
            qspi,
            reset: PinDriver::output(reset)?,
        };

        s.reset().await?;

        Ok(s)
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.reset.set_high()?;

        Timer::after_millis(1).await;

        self.reset.set_low()?;

        Timer::after_millis(20).await;

        self.reset.set_high()?;

        Timer::after_millis(50).await;

        Ok(())
    }

    async fn write_command<'a>(&mut self, cmd: u8, data: &mut [Operation<'a>]) -> Result<()> {
        todo!()
    }
}
