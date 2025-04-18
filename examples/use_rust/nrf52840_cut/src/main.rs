#![no_std]
#![no_main]

#[macro_use]
mod macros;
mod keymap;
mod vial;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{
    self as _, bind_interrupts,
    gpio::{AnyPin, Input, Output},
    interrupt::{self, InterruptExt, Priority},
    peripherals::{self, SAADC},
    saadc::{self, AnyInput, Input as _, Saadc},
    usb::{self, vbus_detect::SoftwareVbusDetect, Driver},
};
use keymap::{COL, NUM_ENCODER, NUM_LAYER, ROW};
use panic_probe as _;
use rmk::{
    ble::SOFTWARE_VBUS,
    channel::EVENT_CHANNEL,
    config::{
        BleBatteryConfig, ControllerConfig, KeyboardUsbConfig, RmkConfig, StorageConfig, VialConfig,
    },
    debounce::default_debouncer::DefaultDebouncer,
    futures::future::join4,
    initialize_encoder_keymap_and_storage, initialize_nrf_sd_and_flash,
    input_device::{
        adc::{AnalogEventType, NrfAdc},
        battery::BatteryProcessor,
        rotary_encoder::{DefaultPhase, RotaryEncoder, RotaryEncoderProcessor},
        Runnable,
    },
    keyboard::Keyboard,
    light::LightController,
    matrix::Matrix,
    run_devices, run_processor_chain, run_rmk,
};
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<peripherals::USBD>;
    SAADC => saadc::InterruptHandler;
});

/// Initializes the SAADC peripheral in single-ended mode on the given pin.
fn init_adc(adc_pin: AnyInput, adc: SAADC) -> Saadc<'static, 1> {
    // Then we initialize the ADC. We are only using one channel in this example.
    let config = saadc::Config::default();
    let channel_cfg = saadc::ChannelConfig::single_ended(adc_pin.degrade_saadc());
    interrupt::SAADC.set_priority(interrupt::Priority::P3);
    let saadc = saadc::Saadc::new(adc, Irqs, config, [channel_cfg]);
    saadc
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello NRF BLE!");
    let mut nrf_config = embassy_nrf::config::Config::default();
    nrf_config.gpiote_interrupt_priority = Priority::P3;
    nrf_config.time_interrupt_priority = Priority::P3;
    interrupt::USBD.set_priority(interrupt::Priority::P2);
    interrupt::CLOCK_POWER.set_priority(interrupt::Priority::P2);
    let p = embassy_nrf::init(nrf_config);
    // Disable external HF clock by default, reduce power consumption
    // info!("Enabling ext hfosc...");
    // ::embassy_nrf::pac::CLOCK.tasks_hfclkstart().write_value(1);
    // while ::embassy_nrf::pac::CLOCK.events_hfclkstarted().read() != 1 {}

    // Pin config
    let (input_pins, output_pins) = config_matrix_pins_nrf!(peripherals: p, input: [P0_09, P0_24, P0_00, P0_13], output:  [P1_11, P0_30, P1_10, P0_10]);

    // Usb config
    let software_vbus = SOFTWARE_VBUS.get_or_init(|| SoftwareVbusDetect::new(true, false));
    let driver = Driver::new(p.USBD, Irqs, software_vbus);

    // Initialize the ADC. We are only using one channel for detecting battery level
    let adc_pin = p.P0_04.degrade_saadc();
    let is_charging_pin = Input::new(AnyPin::from(p.P0_07), embassy_nrf::gpio::Pull::Up);
    let charging_led = Output::new(
        AnyPin::from(p.P0_08),
        embassy_nrf::gpio::Level::Low,
        embassy_nrf::gpio::OutputDrive::Standard,
    );
    let saadc = init_adc(adc_pin, p.SAADC);
    // Wait for ADC calibration.
    saadc.calibrate().await;

    // Keyboard config
    let keyboard_usb_config = KeyboardUsbConfig {
        vid: 0x4c4b,
        pid: 0x4643,
        manufacturer: "JLKB",
        product_name: "CUT",
        serial_number: "vial:f64c2b3c:000001",
    };
    let vial_config = VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF);
    let ble_battery_config =
        BleBatteryConfig::new(Some(is_charging_pin), true, Some(charging_led), false);
    let storage_config = StorageConfig {
        start_addr: 0,
        num_sectors: 6,
        ..Default::default()
    };
    let rmk_config = RmkConfig {
        usb_config: keyboard_usb_config,
        vial_config,
        ble_battery_config,
        storage_config,
        ..Default::default()
    };

    // Initialize the Softdevice and flash
    let (sd, flash) =
        initialize_nrf_sd_and_flash(rmk_config.usb_config.product_name, spawner, None);

    // Initialize the storage and keymap
    let mut default_keymap = keymap::get_default_keymap();
    let mut default_encoder_map = keymap::get_default_encoder_map();
    let (keymap, storage) = initialize_encoder_keymap_and_storage(
        &mut default_keymap,
        &mut default_encoder_map,
        flash,
        rmk_config.storage_config,
        rmk_config.behavior_config.clone(),
    )
    .await;

    // Initialize the matrix + keyboard
    let mut keyboard: Keyboard<'_, ROW, COL, NUM_LAYER, NUM_ENCODER> =
        Keyboard::new(&keymap, rmk_config.behavior_config.clone());
    let debouncer = DefaultDebouncer::<ROW, COL>::new();
    let mut matrix = Matrix::<_, _, _, ROW, COL>::new(input_pins, output_pins, debouncer);
    // let mut matrix = rmk::matrix::TestMatrix::<ROW, COL>::new();

    // Initialize the light controller
    let light_controller: LightController<Output> =
        LightController::new(ControllerConfig::default().light_config);

    let pin_a0 = Input::new(AnyPin::from(p.P0_02), embassy_nrf::gpio::Pull::Up);
    let pin_b0 = Input::new(AnyPin::from(p.P1_13), embassy_nrf::gpio::Pull::Up);
    let mut encoder0 = RotaryEncoder::with_resolution(pin_a0, pin_b0, 4, false, 0);
    
    let pin_a1 = Input::new(AnyPin::from(p.P0_05), embassy_nrf::gpio::Pull::Up);
    let pin_b1 = Input::new(AnyPin::from(p.P1_09), embassy_nrf::gpio::Pull::Up);
    let mut encoder1 = RotaryEncoder::with_resolution(pin_a1, pin_b1, 4, false, 1);
    
    let pin_a2 = Input::new(AnyPin::from(p.P0_28), embassy_nrf::gpio::Pull::Up);
    let pin_b2 = Input::new(AnyPin::from(p.P0_03), embassy_nrf::gpio::Pull::Up);
    let mut encoder2 = RotaryEncoder::with_resolution(pin_a2, pin_b2, 2, false, 2);
    
    let mut adc_device = NrfAdc::new(saadc, [AnalogEventType::Battery], 12000, None);
    let mut batt_proc = BatteryProcessor::new(2000, 2806, &keymap);

    let mut encoder_processor = RotaryEncoderProcessor::new(&keymap);

    // Start
    join4(
        run_devices! (
            (matrix, encoder0,encoder1,encoder2, adc_device) => EVENT_CHANNEL,
        ),
        run_processor_chain! {
            EVENT_CHANNEL => [encoder_processor, batt_proc],
        },
        keyboard.run(), // Keyboard is special
        run_rmk(&keymap, driver, storage, light_controller, rmk_config, sd),
    )
    .await;
}
