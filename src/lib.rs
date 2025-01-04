pub mod net;
use adv_shift_registers::AdvancedShiftRegister;
use anyhow::anyhow;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiBusDriver, SpiDriver};
use esp_idf_svc::sys::EspError;
use smart_leds::{gamma, SmartLedsWrite, RGB};
use std::net::TcpStream;
use std::os::fd::{AsRawFd, IntoRawFd};
use ws2812_spi::Ws2812;

pub struct Displays {
    pub register: AdvancedShiftRegister<2, PinDriver<'static, AnyOutputPin, Output>>,
}

impl Displays {
    pub fn set_number(&mut self, number: u8) {
        self.register.get_shifter_mut(0).set_value(number);
    }
}

const NUM_BASE_LEDS: usize = 15;

/// The LEDs will be configured to have some number as the base then the last one as the beacon
pub struct Leds {
    pub leds: Ws2812<SpiBusDriver<'static, SpiDriver<'static>>>,
}

impl Leds {
    pub fn set_all_colors(&mut self, color: RGB<u8>) {
        self.leds
            .write(gamma(std::iter::repeat_n(color, NUM_BASE_LEDS + 1)))
            .expect("valid led write");
    }
}

#[macro_export]
macro_rules! anyesp {
    ($err: expr) => {{
        let res = $err;
        if res != ::esp_idf_svc::sys::ESP_OK {
            Err(::anyhow::anyhow!("Bad exit code {res}"))
        } else {
            Ok(())
        }
    }};
}

pub fn convert_error(e: EspError) -> anyhow::Error {
    anyhow!("Bad exit code {e}")
}

/// Allows for an async version of the TLS socket
pub struct EspTlsSocket(Option<async_io::Async<TcpStream>>);

impl EspTlsSocket {
    pub const fn new(socket: async_io::Async<TcpStream>) -> Self {
        Self(Some(socket))
    }

    pub fn handle(&self) -> i32 {
        self.0.as_ref().unwrap().as_raw_fd()
    }

    pub fn poll_readable(
        &self,
        ctx: &mut core::task::Context,
    ) -> core::task::Poll<Result<(), esp_idf_svc::sys::EspError>> {
        self.0
            .as_ref()
            .unwrap()
            .poll_readable(ctx)
            .map_err(|_| EspError::from_infallible::<{ esp_idf_svc::sys::ESP_FAIL }>())
    }

    pub fn poll_writeable(
        &self,
        ctx: &mut core::task::Context,
    ) -> core::task::Poll<Result<(), esp_idf_svc::sys::EspError>> {
        self.0
            .as_ref()
            .unwrap()
            .poll_writable(ctx)
            .map_err(|_| EspError::from_infallible::<{ esp_idf_svc::sys::ESP_FAIL }>())
    }

    fn release(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        let socket = self.0.take().unwrap();
        let _ = socket.into_inner().unwrap().into_raw_fd();

        Ok(())
    }
}

impl esp_idf_svc::tls::Socket for EspTlsSocket {
    fn handle(&self) -> i32 {
        EspTlsSocket::handle(self)
    }

    fn release(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        EspTlsSocket::release(self)
    }
}

impl esp_idf_svc::tls::PollableSocket for EspTlsSocket {
    fn poll_readable(
        &self,
        ctx: &mut core::task::Context,
    ) -> core::task::Poll<Result<(), esp_idf_svc::sys::EspError>> {
        EspTlsSocket::poll_readable(self, ctx)
    }

    fn poll_writable(
        &self,
        ctx: &mut core::task::Context,
    ) -> core::task::Poll<Result<(), esp_idf_svc::sys::EspError>> {
        EspTlsSocket::poll_writeable(self, ctx)
    }
}
