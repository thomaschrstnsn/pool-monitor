#![no_std]
#![no_main]

use core::fmt::Debug;
use ds18b20::{Ds18b20, Resolution};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use esp_backtrace as _;
use esp_hal::gpio::any_pin::AnyPin;
use esp_hal::gpio::{Io, Level, OutputOpenDrain, Pull};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{
    clock::ClockControl, delay::Delay, peripherals::Peripherals, prelude::*, system::SystemControl,
};
use heapless::Vec;
use one_wire_bus::{OneWire, OneWireResult};

use esp_wifi::{
    current_millis,
    wifi::{
        utils::create_network_interface, AccessPointInfo, ClientConfiguration, Configuration,
        WifiError, WifiStaDevice,
    },
    wifi_interface::WifiStack,
    EspWifiInitFor,
};
use smoltcp::iface::SocketStorage;

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
        let device_address = device_address.unwrap();

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

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    esp_println::logger::init_logger_from_env();

    // gpio4 is the default pin for the one wire bus
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let pin = AnyPin::new(io.pins.gpio4);
    let od = OutputOpenDrain::new(pin, Level::High, Pull::None);

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
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiStaDevice, &mut socket_set_entries).unwrap();
    let wifi_stack = WifiStack::new(iface, device, sockets, current_millis);

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASS.try_into().unwrap(),
        ..Default::default()
    });
    let res = controller.set_configuration(&client_config);
    log::info!("wifi_set_configuration returned {:?}", res);

    controller.start().unwrap();
    log::info!("is wifi started: {:?}", controller.is_started());

    log::info!("Start Wifi Scan");
    let res: Result<(heapless::Vec<AccessPointInfo, 10>, usize), WifiError> = controller.scan_n();
    if let Ok((res, _count)) = res {
        for ap in res {
            log::info!("{:?}", ap);
        }
    }

    log::info!("{:?}", controller.get_capabilities());
    log::info!("wifi_connect {:?}", controller.connect());

    // wait to get connected
    log::info!("Wait to get connected");
    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                log::error!("{:?}", err);
            }
        }
    }
    log::info!("{:?}", controller.is_connected());

    // wait for getting an ip address
    log::info!("Wait to get an ip address");
    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            log::info!("got ip {:?}", wifi_stack.get_ip_info());
            break;
        }
    }

    let mut one_wire_bus = OneWire::new(od).unwrap();
    let sensors = find_devices(&mut delay, &mut one_wire_bus, ds18b20::FAMILY_CODE);

    let mut c = 0u32;
    loop {
        log::info!("lets go... pool station {c}!");
        match get_temperature(&mut delay, &mut one_wire_bus, &sensors) {
            Ok(_) => {}
            Err(e) => {
                log::error!("Error: {:?}", e);
            }
        };
        c += 1;
        delay.delay(5000.millis());
    }
}
