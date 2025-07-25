use std::sync::{
    atomic::{AtomicU16, AtomicU32},
    mpsc::channel,
    Mutex,
};

use anyhow::anyhow;
use beacons::{
    amoled::{self, Rm690B0},
    anyesp,
    net::{connect_to_network, self_update},
    Displays, Leds, SysTimer,
};
use build_time::build_time_utc;
use embassy_time::Timer;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*, primitives::*};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay::Delay,
        gpio::{AnyInputPin, IOPin, Input, InputPin, OutputPin, PinDriver},
        i2c::{config::Config as I2cConfig, I2cDriver},
        prelude::Peripherals,
        spi::{
            config::{Config, DriverConfig, Duplex, MODE_3},
            SpiBusDriver, SpiDeviceDriver, SpiDriver,
        },
        task::block_on,
        units::Hertz,
    },
    io,
    nvs::EspDefaultNvsPartition,
    sntp, sys,
    timer::EspTaskTimerService,
    wifi::{AsyncWifi, EspWifi},
};
use ft6336::{touch::PointAction, Ft6336};
use log::info;
use pn532::Pn532;
use shared_bus::I2cProxy;
use shiftreg_spi::SipoShiftReg;
use smart_leds::colors::RED;
use ws2812_spi::Ws2812;

async fn amain(
    mut displays: Displays,
    mut leds: Leds,
    mut wifi: AsyncWifi<EspWifi<'static>>,
    mut amoled: Rm690B0<'static, std::sync::Arc<SpiDriver<'static>>>,
    mut touch: Ft6336<I2cProxy<'static, Mutex<I2cDriver<'static>>>>,
    mut touch_irq: PinDriver<'static, AnyInputPin, Input>,
) -> Result<(), anyhow::Error> {
    info!("Main async process started");
    // Red before wifi
    leds.set_all_colors(smart_leds::RGB { r: 100, g: 0, b: 0 });

    // connect_to_network(&mut wifi)
    //     .await
    //     .expect("wifi connection");

    amoled.init().await.expect("init");
    info!("AMOLED init OK");

    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb888::RED)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    // amoled
    //     .bounding_box()
    //     .into_styled(border_stroke)
    //     .draw(&mut amoled)
    //     .expect("box");

    let start: u8 = 0x0;
    for i in start..=start + 0x1F {
        let height = 16;
        let y = i as i32 * height + 10;
        if i % 2 == 0 {
            Rectangle::new(Point::new(50, y), Size::new(50, height as u32))
                .into_styled(PrimitiveStyle::with_fill(Rgb888::WHITE))
                .draw(&mut amoled)?;
        }

        Rectangle::new(Point::new(100, y), Size::new(300, height as u32))
            .into_styled(PrimitiveStyle::with_fill(Rgb888::new(i, i, i)))
            .draw(&mut amoled)?;
    }

    // for i in 1..16 {
    //     for j in 1..16 {
    //         let color: u8 = ((i - 1) * 16) + (j - 1);

    //         let width: i32 = (amoled::WIDTH / 16) as i32;
    //         let height: i32 = (amoled::HEIGHT / 16) as i32;

    //         Rectangle::new(
    //             Point::new((i - 1) as i32 * width, (j - 1) as i32 * height),
    //             Size::new(width as u32, height as u32),
    //         )
    //         .into_styled(PrimitiveStyle::with_fill(Rgb888::new(color, color, color)))
    //         .draw(&mut amoled)?;
    //     }
    // }
    //
    // Rectangle::new(
    //     Point::new(0, 0),
    //     Size::new(amoled::WIDTH as u32, amoled::HEIGHT as u32),
    // )
    // .into_styled(PrimitiveStyle::with_fill(Rgb888::new(100, 255, 100)))
    // .draw(&mut amoled)?;

    info!("Draw done");

    // Timer::after_secs(5).await;

    // Blue before update
    leds.set_all_colors(smart_leds::RGB { r: 0, g: 0, b: 100 });

    // Do this later once I have a build system working
    // self_update(&mut leds).await.expect("self update");

    let (touch_tx, touch_rx) = channel();

    tokio::task::spawn(async move {
        loop {
            touch_irq.wait_for_falling_edge().await.unwrap();
            for touch in touch
                .touch_points_iter()
                .unwrap()
                .filter(|p| matches!(p.action, PointAction::Contact))
            {
                touch_tx.send(touch).unwrap();
            }
        }
    });

    tokio::task::spawn(async move {
        loop {
            while let Ok(point) = touch_rx.try_recv() {
                Rectangle::with_center(
                    Point {
                        x: point.x as i32,
                        y: point.y as i32,
                    },
                    Size::new_equal(20),
                )
                .draw_styled(&PrimitiveStyle::with_fill(Rgb888::WHITE), &mut amoled)
                .unwrap();
            }
            Timer::after_millis(10).await;
        }
    });

    let mut counter = 0_u8;
    loop {
        displays.set_number(Some(counter));
        // info!("BLUE");
        leds.set_all_colors(smart_leds::RGB { r: 0, g: 0, b: 100 });

        // // amoled.all_pixels(true).expect("all pixels on");
        // for i in 1..16 {
        //     for j in 1..16 {
        //         let color: u8 = ((i - 1) * 16) + (j - 1);

        //         let width: i32 = (amoled::WIDTH / 16) as i32;
        //         let height: i32 = (amoled::HEIGHT / 16) as i32;

        //         Rectangle::new(
        //             Point::new((i - 1) as i32 * width, (j - 1) as i32 * height),
        //             Size::new(width as u32, height as u32),
        //         )
        //         .into_styled(PrimitiveStyle::with_fill(Rgb888::new(color, color, color)))
        //         .draw(&mut amoled)?;
        //     }
        // }

        Timer::after_millis(500).await;
        counter = counter.wrapping_add(1);
        displays.set_number(Some(counter));
        // info!("RED");
        leds.set_all_colors(smart_leds::RGB { r: 100, g: 0, b: 0 });

        // amoled.all_pixels(false).expect("all pixels on");
        // Rectangle::new(
        //     Point::new(0, 0),
        //     Size::new(amoled::WIDTH as u32, amoled::HEIGHT as u32),
        // )
        // .into_styled(PrimitiveStyle::with_fill(Rgb888::WHITE))
        // .draw(&mut amoled)?;

        Timer::after_millis(500).await;
        counter = counter.wrapping_add(1);
    }

    Ok(())
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!(
        "Purdue Hackers Beacon Firmware v.{}.{}.{} (Built {})",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
        build_time_utc!()
    );

    let peripherals = Peripherals::take().expect("valid peripherals");

    let driver = SpiDriver::new_quad(
        peripherals.spi3,
        peripherals.pins.gpio18,
        peripherals.pins.gpio17,
        peripherals.pins.gpio16,
        peripherals.pins.gpio14,
        peripherals.pins.gpio8,
        &DriverConfig::new(),
    )
    .expect("qspi driver");

    let driver = std::sync::Arc::new(driver);

    let displays = {
        // let data = PinDriver::output(peripherals.pins.gpio5.downgrade_output()).expect("data pin");
        // let latch = PinDriver::output(peripherals.pins.gpio6.downgrade_output()).expect("data pin");
        // let clk = PinDriver::output(peripherals.pins.gpio9.downgrade_output()).expect("data pin");
        let low_digit = PinDriver::output(peripherals.pins.gpio10).expect("low digit");
        let high_digit = PinDriver::output(peripherals.pins.gpio11).expect("low digit");

        let spi = SpiDeviceDriver::new(
            driver.clone(),
            Some(peripherals.pins.gpio6),
            &Config::default().bit_order(esp_idf_svc::hal::spi::config::BitOrder::MsbFirst),
        )
        .expect("valid sp");

        let register = SipoShiftReg::new(spi);
        Displays::new(register, low_digit, high_digit)
    };

    let amoled = {
        let qspi = SpiDeviceDriver::new(
            driver.clone(),
            Some(peripherals.pins.gpio13),
            &Config::default()
                .data_mode(MODE_3)
                .duplex(Duplex::Half)
                .baudrate(Hertz(80_000_000))
                .queue_size(10),
        )
        .expect("valid qspi");

        Rm690B0::new(qspi, peripherals.pins.gpio39.downgrade_output()).expect("valid amoled")
    };

    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio3,
        peripherals.pins.gpio4,
        &I2cConfig::default().baudrate(Hertz(100_000)),
    )
    .expect("i2c");

    let bus = shared_bus::new_std!(I2cDriver = i2c).expect("i2c bus");

    let (amoled_touch_irq, amoled_touch) = {
        let mut reset = PinDriver::output(peripherals.pins.gpio5).expect("reset");
        reset.set_low().expect("reset low");
        std::thread::sleep(std::time::Duration::from_millis(5));
        reset.set_high().expect("reset high");

        let mut irq = PinDriver::input(peripherals.pins.gpio9.downgrade_input()).expect("irq");
        irq.set_interrupt_type(esp_idf_svc::hal::gpio::InterruptType::NegEdge)
            .expect("set irq mode");

        let mut touch = Ft6336::new(bus.acquire_i2c());
        touch.init().expect("touch init");
        touch.interrupt_by_state().expect("interrupt");
        (irq, touch)
    };

    let (nfc_irq, nfc) = {
        let spi = SpiDeviceDriver::new(
            driver,
            Some(peripherals.pins.gpio38),
            &Config::default()
                .bit_order(esp_idf_svc::hal::spi::config::BitOrder::LsbFirst)
                .duplex(Duplex::Full),
        )
        .expect("valid spi");

        let irq = PinDriver::input(peripherals.pins.gpio12).expect("irq pin");

        let interface = pn532::i2c::I2CInterface {
            i2c: bus.acquire_i2c(),
        };

        let nfc = Pn532::<_, _, 64>::new(interface, SysTimer::default());

        (irq, nfc)
    };

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let wifi = AsyncWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs)).unwrap(),
        sys_loop,
        EspTaskTimerService::new().unwrap(),
    )
    .expect("wifi");

    info!("Wifi init OK");

    let _sntp = sntp::EspSntp::new_default().unwrap();

    info!("SNTP init OK");

    // todo!("NFC Initialization for passports");

    std::thread::Builder::new()
        .stack_size(60_000)
        .spawn(|| {
            let leds = {
                let driver = SpiDriver::new_without_sclk(
                    peripherals.spi2,
                    peripherals.pins.gpio15,
                    None::<AnyInputPin>,
                    &DriverConfig::new(),
                )
                .expect("valid spi");
                let cfg = Config::new().baudrate(Hertz(1_500_000));
                let bus = SpiBusDriver::new(driver, &cfg).expect("valid spi bus");

                let leds = Ws2812::new(bus);
                Leds { leds }
            };

            anyesp!(unsafe {
                sys::esp_vfs_eventfd_register(&sys::esp_vfs_eventfd_config_t { max_fds: 16 })
            })
            .unwrap();

            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(amain(
                    displays,
                    leds,
                    wifi,
                    amoled,
                    amoled_touch,
                    amoled_touch_irq,
                ))
                .expect("amain ok")
        })
        .unwrap()
        .join()
        .unwrap();
}
