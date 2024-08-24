#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Io, Level, OutputOpenDrain, Pull},
    peripherals::Peripherals,
    prelude::*,
    rng::Rng,
    system::SystemControl,
    timer::timg::TimerGroup,
};

mod channel;
mod http;
mod sensors;

use esp_wifi::{
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiInitFor,
};

#[repr(C)]
struct InitParams {
    // nullptr to use default configuration of pins,
    // otherwise pointer to pin numbers for
    // R1_PIN
    // G1_PIN
    // B1_PIN
    // R2_PIN
    // G2_PIN
    // B2_PIN
    // A_PIN
    // B_PIN
    // C_PIN
    // D_PIN
    // E_PIN
    // LAT_PIN
    // OE_PIN
    // CLK_PIN
    pins: *mut i8,
}
// struct DrawParams
// {
//     double PoolIn;
//     double PoolInDeltaT;
//     double Boiler;
//     double HeatExchangerIn;
//     double HeatExchangerOut;
// };

#[link(name = "poolstationscreen")]
extern "C" {
    // void poolScreenInit(const InitParams* params);
    //
    // void poolScreenDraw(const DrawParams* params);
    //
    // void poolScreenClear();
    // void poolScreenLog(const char* text);

    fn poolScreenClear();
    fn poolScreenInit(init: *const InitParams);
}

const SSID: &str = env!("ESP32_WIFI_SSID");
const PASS: &str = env!("ESP32_WIFI_PASS");

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
    unsafe {
        let i: InitParams = InitParams {
            pins: core::ptr::null_mut(),
        };
        poolScreenInit(&i);
    }

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

    let seed = 420 ^ 69 ^ 313373 ^ 0xCAFEBABE; // very random, very secure seed

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
    spawner.spawn(net_task(stack)).ok();

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

    let pin = io.pins.gpio5; // gpio4 is the default pin for the one wire bus
                             // let all_pins = [io.pins.gpio0, io.pins.gpio1];
    let ood = OutputOpenDrain::new(pin, Level::High, Pull::None);
    spawner.spawn(sensors::read_sensors(ood, delay)).ok();
    spawner.spawn(http::post_updates(stack)).ok();
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    log::info!("start connection task");
    log::info!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        if let WifiState::StaConnected = esp_wifi::wifi::get_wifi_state() {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
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
