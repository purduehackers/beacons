use adv_shift_registers::AdvancedShiftRegister;
use beacons::{
    amoled::Rm690B0,
    anyesp,
    net::{connect_to_network, self_update},
    Displays, Leds,
};
use build_time::build_time_utc;
use embassy_time::Timer;
use embedded_graphics::{pixelcolor::Rgb888, prelude::*, primitives::*};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay::Delay,
        gpio::{AnyInputPin, IOPin, OutputPin, PinDriver},
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
use log::info;
use ws2812_spi::Ws2812;

async fn amain(
    mut displays: Displays,
    mut leds: Leds,
    mut wifi: AsyncWifi<EspWifi<'static>>,
    mut amoled: Rm690B0<'static, SpiDriver<'static>>,
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

    info!("Draw done");
    Timer::after_secs(5).await;

    // Blue before update
    leds.set_all_colors(smart_leds::RGB { r: 0, g: 0, b: 100 });

    // Do this later once I have a build system working
    // self_update(&mut leds).await.expect("self update");

    let mut counter = 0_u8;
    loop {
        displays.set_number(Some(counter));
        // info!("BLUE");
        leds.set_all_colors(smart_leds::RGB { r: 0, g: 0, b: 100 });

        // amoled.all_pixels(true).expect("all pixels on");

        Timer::after_secs(1).await;
        counter = counter.wrapping_add(1);
        displays.set_number(Some(counter));
        // info!("RED");
        leds.set_all_colors(smart_leds::RGB { r: 100, g: 0, b: 0 });

        // amoled.all_pixels(false).expect("all pixels on");

        Timer::after_secs(1).await;
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

    let displays = {
        let data = PinDriver::output(peripherals.pins.gpio5.downgrade_output()).expect("data pin");
        let latch = PinDriver::output(peripherals.pins.gpio6.downgrade_output()).expect("data pin");
        let clk = PinDriver::output(peripherals.pins.gpio9.downgrade_output()).expect("data pin");
        let low_digit = PinDriver::output(peripherals.pins.gpio10).expect("low digit");
        let high_digit = PinDriver::output(peripherals.pins.gpio11).expect("low digit");

        let register = AdvancedShiftRegister::new(data, clk, latch, 0);
        Displays::new(register, low_digit, high_digit)
    };

    let amoled = {
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

        let qspi = SpiDeviceDriver::new(
            driver,
            Some(peripherals.pins.gpio13),
            &Config::default()
                .data_mode(MODE_3)
                .duplex(Duplex::Half)
                .baudrate(Hertz(80_000_000)),
        )
        .expect("valid qspi");

        Rm690B0::new(qspi, peripherals.pins.gpio39.downgrade_output()).expect("valid amoled")
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
            block_on(amain(displays, leds, wifi, amoled)).expect("amain ok")
        })
        .unwrap()
        .join()
        .unwrap();
}
