#![feature(super_let)]
pub mod net;
use adv_shift_registers::wrappers::ShifterPin;
use adv_shift_registers::AdvancedShiftRegister;
use anyhow::anyhow;
use embassy_time::Timer;
use esp_idf_svc::hal::delay::Delay;
use esp_idf_svc::hal::gpio::{
    AnyOutputPin, Gpio10, Gpio11, Gpio4, InputPin, Output, OutputPin, PinDriver,
};
use esp_idf_svc::hal::spi::{SpiBusDriver, SpiDriver};
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::sys::EspError;
use log::info;
use seven_segment::{SevenSegment, SevenSegmentPins};
use smart_leds::{gamma, SmartLedsWrite, RGB};
use std::net::TcpStream;
use std::ops::DerefMut;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::sync::mpsc;
use ws2812_spi::Ws2812;

pub mod amoled;

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SetNumber(Option<u8>),
}

pub struct Displays {
    pub messenger: mpsc::Sender<DisplayCommand>,
}

async fn display_thread(
    mut register: AdvancedShiftRegister<2, PinDriver<'static, AnyOutputPin, Output>>,
    mut low_digit: PinDriver<'static, Gpio10, Output>,
    mut high_digit: PinDriver<'static, Gpio11, Output>,
    rx: mpsc::Receiver<DisplayCommand>,
) {
    let a = register.get_pin_mut(0, 0, false);
    let b = register.get_pin_mut(0, 1, false);
    let c = register.get_pin_mut(0, 2, false);
    let d = register.get_pin_mut(0, 3, false);
    let e = register.get_pin_mut(0, 4, false);
    let f = register.get_pin_mut(0, 5, false);
    let g = register.get_pin_mut(0, 6, false);
    register.get_shifter_mut(0).set_value(0xff);

    let mut seg = SevenSegmentPins {
        a,
        b,
        c,
        d,
        e,
        f,
        g,
    }
    .with_common_anode();

    let mut num_high = Some(9);
    let mut num_low = Some(9);
    loop {
        match rx.try_recv() {
            Ok(msg) => match msg {
                DisplayCommand::SetNumber(n) => match n {
                    Some(n) => {
                        num_high = Some(n >> 4);
                        num_low = Some(n & 0x0F);
                    }
                    None => {
                        num_high = None;
                        num_low = None;
                    }
                },
            },
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => return,
        }

        const SEG_DELAY: u64 = 3;
        if let Some(high) = num_high {
            seg.set(high).unwrap();
            register.update_shifters();
            high_digit.set_high().unwrap();
            Timer::after_millis(SEG_DELAY).await;
            high_digit.set_low().unwrap();
        }

        if let Some(low) = num_low {
            seg.set(low).unwrap();
            register.update_shifters();
            low_digit.set_high().unwrap();
            Timer::after_millis(SEG_DELAY).await;
            low_digit.set_low().unwrap();
        }
    }
}

impl Displays {
    pub fn new(
        register: AdvancedShiftRegister<2, PinDriver<'static, AnyOutputPin, Output>>,
        mut low_digit: PinDriver<'static, Gpio10, Output>,
        mut high_digit: PinDriver<'static, Gpio11, Output>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(|| {
            block_on(display_thread(register, low_digit, high_digit, rx));
        });
        Self { messenger: tx }
    }

    pub fn set_number(&mut self, number: Option<u8>) {
        self.messenger
            .send(DisplayCommand::SetNumber(number))
            .expect("valid send");
    }
}

const NUM_BASE_LEDS: usize = 5;

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
