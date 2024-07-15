#![no_std]
#![no_main]

use core::fmt::Debug;
use ds18b20::{Ds18b20, Resolution};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{GpioPin, Io, Level, OutputOpenDrain, Pull},
    peripherals::Peripherals,
    prelude::*,
    rng::Rng,
    system::SystemControl,
    timer::timg::TimerGroup,
};
use heapless::Vec;
use one_wire_bus::{OneWire, OneWireResult};

use esp_wifi::{
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiInitFor,
};

fn find_devices<P, E>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
    family_code: u8,
) -> Vec<Ds18b20, 2>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    let mut devices = Vec::new();
    for device_address in one_wire_bus.devices(false, delay) {
        // The search could fail at any time, so check each result. The iterator automatically
        // ends after an error.
        let device_address = device_address.expect("scanning one-wire-bus for devices");

        if device_address.family_code() == family_code {
            // The family code can be used to identify the type of device
            // If supported, another crate can be used to interact with that device at the given address
            log::info!(
                "Found device at address {:?} with family code: {:#x?}",
                device_address,
                device_address.family_code()
            );

            let sensor = match Ds18b20::new::<E>(device_address) {
                Ok(sensor) => sensor,
                Err(e) => {
                    log::error!("Error creating sensor: {:?}", e);
                    panic!("oh no")
                }
            };
            if let Err(x) = devices.push(sensor) {
                log::warn!(
                    "found more sensors than expected, discarding... {:?}",
                    x.address()
                );
                break;
            }
        }
    }

    devices
}

fn get_temperature<P, E>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
    sensors: &[Ds18b20],
) -> OneWireResult<(), E>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    // initiate a temperature measurement for all connected devices
    ds18b20::start_simultaneous_temp_measurement(one_wire_bus, delay)?;

    // wait until the measurement is done. This depends on the resolution you specified
    // If you don't know the resolution, you can obtain it from reading the sensor data,
    // or just wait the longest time, which is the 12-bit resolution (750ms)
    Resolution::Bits12.delay_for_measurement_time(delay);

    // contains the read temperature, as well as config info such as the resolution used
    for sensor in sensors {
        let sensor_data = sensor.read_data(one_wire_bus, delay)?;
        log::info!(
            "Device at {:?} is {}°C",
            sensor.address(),
            sensor_data.temperature
        );
    }

    Ok(())
}

const SSID: &str = env!("ESP32_WIFI_SSID");
const PASS: &str = env!("ESP32_WIFI_PASS");
//
// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();

    // wifi
    let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;
    let init = esp_wifi::initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let timer_group0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timer_group0);

    let config = Config::dhcpv4(Default::default());

    let seed = 420_691_337; // very random, very secure seed

    // Init network stack
    let stack = &*mk_static!(
        Stack<WifiDevice<'_, WifiStaDevice>>,
        Stack::new(
            wifi_interface,
            config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed
        )
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(&stack)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let delay = Delay::new(&clocks);

    let pin = io.pins.gpio4; // gpio4 is the default pin for the one wire bus
    let ood = OutputOpenDrain::new(pin, Level::High, Pull::None);
    spawner.spawn(read_sensors(ood, delay)).ok();
}

#[embassy_executor::task]
async fn read_sensors(ood: OutputOpenDrain<'static, GpioPin<4>>, mut delay: Delay) {
    let mut one_wire_bus = OneWire::new(ood).expect("creating a one-wire-bus");
    let sensors = find_devices(&mut delay, &mut one_wire_bus, ds18b20::FAMILY_CODE);
    let mut c = 0u32;
    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        log::info!("lets go... pool station {c}!");
        match get_temperature(&mut delay, &mut one_wire_bus, &sensors) {
            Ok(_) => {}
            Err(e) => {
                log::error!("Error getting sensor temperature: {:?}", e);
            }
        };
        c += 1;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    log::info!("start connection task");
    log::info!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASS.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            log::info!("Starting wifi");
            controller.start().await.unwrap();
            log::info!("Wifi started!");
        }
        log::info!("About to connect...");

        match controller.connect().await {
            Ok(_) => log::info!("Wifi connected!"),
            Err(e) => {
                log::error!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
