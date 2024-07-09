#![no_std]
#![no_main]

use core::fmt::Debug;
use ds18b20::{Ds18b20, Resolution};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use esp_backtrace as _;
use esp_hal::gpio::any_pin::AnyPin;
use esp_hal::gpio::{Io, Level, OutputOpenDrain, Pull};
use esp_hal::{
    clock::ClockControl, delay::Delay, peripherals::Peripherals, prelude::*, system::SystemControl,
};
use one_wire_bus::{OneWire, OneWireResult};

fn find_devices<P, E>(delay: &mut impl DelayNs, one_wire_pin: P)
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    let mut one_wire_bus = OneWire::new(one_wire_pin).unwrap();
    for device_address in one_wire_bus.devices(false, delay) {
        // The search could fail at any time, so check each result. The iterator automatically
        // ends after an error.
        let device_address = device_address.unwrap();

        // The family code can be used to identify the type of device
        // If supported, another crate can be used to interact with that device at the given address
        log::info!(
            "Found device at address {:?} with family code: {:#x?}",
            device_address,
            device_address.family_code()
        )
    }
}

fn get_temperature<P, E>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
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

    // iterate over all the devices, and report their temperature
    let mut search_state = None;
    loop {
        if let Some((device_address, state)) =
            one_wire_bus.device_search(search_state.as_ref(), false, delay)?
        {
            search_state = Some(state);
            if device_address.family_code() != ds18b20::FAMILY_CODE {
                // skip other devices
                continue;
            }
            // You will generally create the sensor once, and save it for later
            let sensor = Ds18b20::new(device_address)?;

            // contains the read temperature, as well as config info such as the resolution used
            let sensor_data = sensor.read_data(one_wire_bus, delay)?;
            log::info!(
                "Device at {:?} is {}°C",
                device_address,
                sensor_data.temperature
            );
        } else {
            break;
        }
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

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let mut pin = AnyPin::new(io.pins.gpio4);
    let mut od = OutputOpenDrain::new(pin, Level::High, Pull::None);
    // let mut pin = io.pins.gpio4;

    esp_println::logger::init_logger_from_env();

    find_devices(&mut delay, &mut od);

    let mut one_wire_bus = OneWire::new(od).unwrap();

    let mut c = 0u32;
    loop {
        log::info!("lets go... pool station {c}!");
        get_temperature(&mut delay, &mut one_wire_bus).expect("have value");
        c += 1;
        delay.delay(5000.millis());
    }
}
