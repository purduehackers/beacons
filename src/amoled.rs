use anyhow::Result;
use embassy_time::Timer;
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

        Operation::Write(&buf)
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
        let cmd = (cmd as u16) << 8;
        todo!()
    }

    async fn manufacturer_init(&mut self) -> Result<()> {
        let mut ops = command_ops![0xFE, write_buf!([0x20])];

        self.qspi.transaction_async(&mut ops).await?;

        Ok(())
    }
}
