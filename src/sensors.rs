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
use one_wire_bus::{OneWire, OneWireError, OneWireResult};

use crate::channel::TEMP_CHANNEL;

const NUM_SENSORS: usize = 2;

type Sensors = Vec<Ds18b20, NUM_SENSORS>;

#[derive(Debug, Clone)]
pub struct Reading {
    pub temperature_celcius: f32,
    pub sensor_address: one_wire_bus::Address,
}

pub type TempMessage = [Reading; NUM_SENSORS];

fn find_devices<P, E>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
    family_code: u8,
) -> Result<Sensors, OneWireError<E>>
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    let mut devices = Vec::new();
    for device_address in one_wire_bus.devices(false, delay) {
        // The search could fail at any time, so check each result. The iterator automatically
        // ends after an error.
        let device_address = device_address?;

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

    assert!(devices.len() == NUM_SENSORS);

    Ok(devices)
}

async fn find_devices_retry<P, E>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
    family_code: u8,
) -> Sensors
where
    P: OutputPin<Error = E> + InputPin<Error = E>,
    E: Debug,
{
    let mut retry = 0;
    loop {
        match find_devices(delay, one_wire_bus, family_code) {
            Ok(devices) => return devices,
            Err(e) => {
                log::warn!("Error finding devices: {:?}", e);
                retry += 1;
                if retry < 3 {
                    Timer::after(Duration::from_millis(25)).await;
                    continue;
                }
                log::error!("giving up finding devices");
                panic!("oh no")
            }
        }
    }
}

async fn get_temperature<P, E, const N: usize>(
    delay: &mut impl DelayNs,
    one_wire_bus: &mut OneWire<P>,
    sensors: &Vec<Ds18b20, N>,
) -> OneWireResult<Vec<Reading, N>, E>
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

    let mut result = Vec::new();

    for sensor in sensors {
        let mut retry = 0;
        loop {
            let res = match sensor.read_data(one_wire_bus, delay) {
                Ok(sensor_data) => {
                    log::info!(
                        "Device at {:?} is {}°C (retry {})",
                        sensor.address(),
                        sensor_data.temperature,
                        retry
                    );
                    let _ = result.push(Reading {
                        sensor_address: *sensor.address(),
                        temperature_celcius: sensor_data.temperature,
                    });
                    Ok(())
                }
                Err(e) => {
                    log::warn!("error getting temp: {:?}", e);
                    retry += 1;
                    if retry < 3 {
                        Timer::after(Duration::from_millis(25)).await;
                        continue;
                    }
                    Err(e)
                }
            };
            res?;
            break;
        }
    }

    Ok(result)
}

#[embassy_executor::task]
pub async fn read_sensors(ood: OutputOpenDrain<'static, GpioPin<4>>, mut delay: Delay) {
    let mut one_wire_bus = OneWire::new(ood).expect("creating a one-wire-bus");

    let publisher = TEMP_CHANNEL
        .dyn_publisher()
        .expect("getting publisher for channel");

    let sensors = find_devices_retry(&mut delay, &mut one_wire_bus, ds18b20::FAMILY_CODE).await;

    let mut c = 0u32;
    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        log::info!("lets go... pool station {c}!");
        match get_temperature(&mut delay, &mut one_wire_bus, &sensors).await {
            Ok(readings) => match readings.into_array() {
                Ok(readings) => {
                    let _ = publisher.publish(readings).await;
                }
                _ => {
                    log::error!("could not convert to array of readings");
                }
            },
            Err(e) => {
                log::error!("Error getting sensor temperature: {:?}", e);
            }
        };
        c += 1;
    }
}
