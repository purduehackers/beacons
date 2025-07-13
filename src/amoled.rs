use anyhow::Result;
use embassy_time::Timer;
use embedded_graphics::prelude::{OriginDimensions, RgbColor};
use embedded_graphics_core::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, Size},
    primitives::Rectangle,
    Pixel,
};
use log::info;
use std::{
    borrow::Borrow,
    ops::{Range, RangeInclusive},
};

use esp_idf_svc::hal::{
    gpio::{AnyOutputPin, Output, Pin, PinDriver},
    prelude::*,
    spi::{config::LineWidth, *},
    task::*,
};

use crate::anyesp;

pub const WIDTH: usize = 450;
pub const HEIGHT: usize = 600;

pub struct Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    qspi: SpiDeviceDriver<'d, T>,
    reset: PinDriver<'d, AnyOutputPin, Output>,
}

macro_rules! command_ops {
    ($header: expr; $cmd: expr, $($itms:expr),*) => {
        {
        super let header: [u8; 1] = [$header];
        super let cmd_addr: [u8; 3] = [0x00, $cmd, 0x00];
        [
            Operation::WriteWithWidth(&header, ::esp_idf_svc::hal::spi::config::LineWidth::Single),
            Operation::WriteWithWidth(&cmd_addr, ::esp_idf_svc::hal::spi::config::LineWidth::Single),
            $($itms),*
        ]
        }
    };
    ($cmd: expr$(, $itms:expr)*) => {
        command_ops![0x02; $cmd, $($itms),*]
    };
    (p: $cmd: expr$(, $itms:expr)*) => {
        command_ops![0x32; $cmd, $($itms),*]
    };
}

macro_rules! write_buf {
    ($buf: expr) => {{
        super let buf = $buf;

        Operation::WriteWithWidth(&buf, ::esp_idf_svc::hal::spi::config::LineWidth::Single)
    }};

    (q: $buf: expr) => {{
        super let buf = $buf;

        Operation::WriteWithWidth(&buf, ::esp_idf_svc::hal::spi::config::LineWidth::Quad)
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
            // buf: [0; WIDTH * HEIGHT * 3],
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

        // Tearing Effect off
        self.qspi.transaction(&mut command_ops![0x34])?;

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

        // Brightness
        // Max is 0xFF but only using 50% for Feather since regulator is kinda weak
        let mut ops = command_ops![0x51, write_buf!([0x80])];
        self.qspi.transaction(&mut ops)?;

        Ok(())
    }

    pub fn all_pixels(&mut self, on: bool) -> Result<()> {
        self.qspi
            .transaction(&mut command_ops![if on { 0x23 } else { 0x22 }])?;

        Ok(())
    }

    pub fn set_column_range(&mut self, mut col: RangeInclusive<u16>) -> Result<()> {
        let fudge_s = if col.start() % 2 == 0 {
            *col.start()
        } else {
            col.start() - 1
        };
        let fudge_e = if col.end() % 2 == 1 {
            *col.end()
        } else {
            col.end() - 1
        };

        col = fudge_s + 16..=fudge_e + 16;

        self.qspi.transaction(&mut command_ops![
            0x2A,
            write_buf!([
                (col.start() >> 8) as u8,
                (col.start() & 0xFF) as u8,
                (col.end() >> 8) as u8,
                (col.end() & 0xFF) as u8
            ])
        ])?;

        Ok(())
    }

    pub fn set_row_range(&mut self, row: RangeInclusive<u16>) -> Result<()> {
        let fudge_s = if row.start() % 2 == 0 {
            *row.start()
        } else {
            row.start() - 1
        };
        let fudge_e = if row.end() % 2 == 1 {
            *row.end()
        } else {
            row.end() - 1
        };

        let row = fudge_s..=fudge_e;

        self.qspi.transaction(&mut command_ops![
            0x2B,
            write_buf!([
                (row.start() >> 8) as u8,
                (row.start() & 0xFF) as u8,
                (row.end() >> 8) as u8,
                (row.end() & 0xFF) as u8
            ])
        ])?;

        Ok(())
    }

    pub fn write_pixels(&mut self, pixels: &[u8]) -> Result<()> {
        self.write_pixels_from_iterator(pixels.iter().cloned())
    }

    pub fn write_pixels_from_iterator<I>(&mut self, pixels: I) -> Result<()>
    where
        I: IntoIterator<Item = u8>,
    {
        self.qspi.transaction(&mut command_ops![0x2C])?;

        let header = [0x32_u8, 0x00, 0x2C, 0x00];
        let mut pixels = header.into_iter().chain(pixels.into_iter()).peekable();

        {
            let mut header_consumed = false;
            let handle = self.qspi.device();
            let mut buf = [0u8; 64];

            use esp_idf_svc::sys::*;
            struct BusLock(spi_device_handle_t);

            impl BusLock {
                fn new(device: spi_device_handle_t) -> Result<Self, EspError> {
                    use esp_idf_svc::hal::delay::BLOCK;
                    esp!(unsafe { spi_device_acquire_bus(device, BLOCK) })?;

                    Ok(Self(device))
                }
            }

            impl Drop for BusLock {
                fn drop(&mut self) {
                    unsafe {
                        spi_device_release_bus(self.0);
                    }
                }
            }

            let mut lock = None;
            loop {
                let mut offset = 0usize;
                let buf_max = if header_consumed {
                    buf.len()
                } else {
                    header.len()
                };
                while offset < buf_max {
                    if let Some(word) = pixels.next() {
                        buf[offset] = word;
                        offset += 1;
                    } else {
                        break;
                    }
                }
                // info!("OFFSET {offset}");

                if offset == 0 {
                    break;
                }

                let mut transaction = spi_transaction_t {
                    __bindgen_anon_1: spi_transaction_t__bindgen_ty_1 {
                        tx_buffer: buf.as_ptr() as *const _,
                    },
                    __bindgen_anon_2: spi_transaction_t__bindgen_ty_2 {
                        rx_buffer: core::ptr::null_mut() as *mut _,
                    },
                    flags: if header_consumed {
                        SPI_TRANS_MODE_QIO
                    } else {
                        0
                    },
                    length: offset * 8,
                    rxlength: 0,
                    ..Default::default()
                };

                if pixels.peek().is_some() {
                    if lock.is_none() {
                        lock = Some(BusLock::new(handle)?);
                    }

                    transaction.flags |= SPI_TRANS_CS_KEEP_ACTIVE;
                }

                unsafe {
                    esp!(spi_device_polling_transmit(
                        handle,
                        &mut transaction as *mut _
                    ))?;
                }

                header_consumed = true;
            }
        }

        self.qspi.transaction(&mut command_ops![p: 0x00])?;

        self.qspi.transaction(&mut command_ops![0x29])?;

        Ok(())
    }
}

impl<'d, T> OriginDimensions for Rm690B0<'d, T>
where
    T: Borrow<SpiDriver<'d>> + 'd,
{
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
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
        let bb = self.bounding_box();
        for Pixel(point, color) in pixels.into_iter().filter(|Pixel(p, _)| bb.contains(*p)) {
            let y = point.y as u16;
            let x = point.x as u16;
            info!("POINT {x}, {y}: {color:?}");
            self.set_row_range(y..=y)?;
            self.set_column_range(x..=x)?;
            self.write_pixels(&[color.r(), color.g(), color.b()])?;
        }

        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &Rectangle,
        colors: I,
    ) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let drawable = area.intersection(&self.bounding_box());
        if drawable.is_zero_sized() {
            return Ok(());
        }

        let x1 = drawable.top_left.x;
        let x2 = drawable.bottom_right().expect("br").x;
        let y1 = drawable.top_left.y;
        let y2 = drawable.bottom_right().expect("br").y;

        self.set_column_range(x1 as u16..=x2 as u16)?;
        self.set_row_range(y1 as u16..=y2 as u16)?;

        self.write_pixels_from_iterator(
            colors
                .into_iter()
                .take(drawable.size.width as usize * drawable.size.height as usize)
                .map(|p| [p.r(), p.g(), p.b()])
                .flatten(),
        )?;

        Ok(())
    }
}
