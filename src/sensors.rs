use core::fmt::Debug;
use ds18b20::{Ds18b20, Resolution};
use embassy_time::{Duration, Timer};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{GpioPin, OutputOpenDrain},
};
use heapless::Vec;
use one_wire_bus::{OneWire, OneWireResult};

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

#[embassy_executor::task]
pub async fn read_sensors(ood: OutputOpenDrain<'static, GpioPin<4>>, mut delay: Delay) {
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
