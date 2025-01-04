use adv_shift_registers::AdvancedShiftRegister;
use beacons::{
    net::{connect_to_network, self_update},
    Displays, Leds,
};
use build_time::build_time_utc;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{AnyInputPin, OutputPin, PinDriver},
        prelude::Peripherals,
        spi::{config::DriverConfig, SpiBusDriver, SpiDriver},
        task::block_on,
    },
    io,
    nvs::EspDefaultNvsPartition,
    sntp,
    timer::EspTaskTimerService,
    wifi::{AsyncWifi, EspWifi},
};
use log::info;
use ws2812_spi::Ws2812;

async fn amain(displays: Displays, mut leds: Leds, mut wifi: AsyncWifi<EspWifi<'static>>) {
    // Red before wifi
    leds.set_all_colors(smart_leds::RGB { r: 255, g: 0, b: 0 });

    connect_to_network(&mut wifi)
        .await
        .expect("wifi connection");

    // Blue before update
    leds.set_all_colors(smart_leds::RGB { r: 0, g: 0, b: 255 });

    self_update(&mut leds).await.expect("self update");

    todo!()
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

        let register = AdvancedShiftRegister::new(data, clk, latch, 0);
        Displays { register }
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
                let bus = SpiBusDriver::new(driver, &Default::default()).expect("valid spi bus");

                let leds = Ws2812::new(bus);
                Leds { leds }
            };

            io::vfs::initialize_eventfd(5).unwrap();
            block_on(amain(displays, leds, wifi))
        })
        .unwrap()
        .join()
        .unwrap();
}
