#[cfg(feature = "_ble")]
use core::cell::Cell;

#[cfg(feature = "_ble")]
use embassy_sync::blocking_mutex::Mutex;
use embedded_hal::digital::InputPin;
use rmk_macro::{input_device, processor};
#[cfg(feature = "_ble")]
use rmk_types::battery::{BatteryStatus, ChargeState};

#[cfg(feature = "_ble")]
use crate::RawMutex;
#[cfg(feature = "_ble")]
use crate::event::BatteryStatusEvent;
use crate::event::{BatteryAdcEvent, ChargingStateEvent, publish_event};

/// Cached battery status, updated by [`BatteryProcessor::commit`] alongside every
/// [`BatteryStatusEvent`] publish so host services can read the current value
/// synchronously without subscribing to the event stream.
#[cfg(feature = "_ble")]
pub(crate) static BATTERY_STATUS: Mutex<RawMutex, Cell<BatteryStatus>> =
    Mutex::new(Cell::new(BatteryStatus::Unavailable));

#[cfg(feature = "_ble")]
pub(crate) fn current_battery_status() -> BatteryStatus {
    BATTERY_STATUS.lock(|c| c.get())
}

/// Reads charging state from a GPIO pin and publishes ChargingStateEvent.
///
/// This input device monitors a charging state pin and publishes events when
/// the charging state changes.
#[input_device(publish = ChargingStateEvent)]
pub struct ChargingStateReader<I: InputPin> {
    // Charging state pin or standby pin
    state_input: I,
    // True: low represents charging, False: high represents charging
    low_active: bool,
    // True: charging, False: not charging
    current_charging_state: bool,
    // First read done
    first_read: bool,
}

impl<I: InputPin> ChargingStateReader<I> {
    pub fn new(state_input: I, low_active: bool) -> Self {
        Self {
            state_input,
            low_active,
            current_charging_state: false,
            first_read: false,
        }
    }

    /// Read the charging state and return an event.
    /// This method waits until there's a state change to report.
    async fn read_charging_state_event(&mut self) -> ChargingStateEvent {
        // For the first read, don't check whether the charging state is changed
        if !self.first_read {
            // Wait 2s before reading the first value
            embassy_time::Timer::after_secs(2).await;
            let charging_state = if self.low_active {
                self.state_input.is_low().unwrap_or(false)
            } else {
                self.state_input.is_high().unwrap_or(false)
            };
            self.current_charging_state = charging_state;
            self.first_read = true;
            return ChargingStateEvent {
                charging: charging_state,
            };
        }

        loop {
            // Check charging state every 5 seconds
            embassy_time::Timer::after_secs(5).await;

            // Detect charging state
            let charging_state = if self.low_active {
                self.state_input.is_low().unwrap_or(false)
            } else {
                self.state_input.is_high().unwrap_or(false)
            };

            // Only return event when charging state changes
            if charging_state != self.current_charging_state {
                self.current_charging_state = charging_state;
                return ChargingStateEvent {
                    charging: charging_state,
                };
            }
        }
    }
}

/// BatteryProcessor processes battery adc value and charging state,
/// emits `BatteryStatusEvent` when battery status changes.
#[processor(subscribe = [BatteryAdcEvent, ChargingStateEvent])]
pub struct BatteryProcessor {
    adc_divider_measured: u32,
    adc_divider_total: u32,
    /// Current battery status
    battery_status: BatteryStatus,
}

impl BatteryProcessor {
    pub fn new(adc_divider_measured: u32, adc_divider_total: u32) -> Self {
        BatteryProcessor {
            adc_divider_measured,
            adc_divider_total,
            battery_status: BatteryStatus::Unavailable,
        }
    }

    /// Apply a new battery status: persist on the processor, mirror into
    /// [`BATTERY_STATUS`] for synchronous readers, and broadcast via
    /// [`BatteryStatusEvent`].
    #[cfg(feature = "_ble")]
    fn commit(&mut self, status: BatteryStatus) {
        self.battery_status = status;
        BATTERY_STATUS.lock(|c| c.set(status));
        publish_event(BatteryStatusEvent::from(status));
    }

    #[cfg(feature = "_ble")]
    fn get_battery_percent(&self, val: u16) -> u8 {
        if self.adc_divider_measured == 0 || self.adc_divider_total == 0 {
            error!("Battery ADC divider values must be greater than zero");
            return 0;
        }
        let battery_mv = u64::from(val) * u64::from(self.adc_divider_total) / u64::from(self.adc_divider_measured);
        (battery_mv.saturating_sub(3600) / 6).min(100) as u8
    }
}

impl BatteryProcessor {
    async fn on_battery_adc_event(&mut self, event: BatteryAdcEvent) {
        let val = event.0;
        trace!("Detected battery ADC value: {:?}", val);

        #[cfg(feature = "_ble")]
        match self.battery_status {
            // Skip ADC updates while charging
            BatteryStatus::Available {
                charge_state: ChargeState::Charging,
                ..
            } => {}
            // Not charging: publish if the percentage changed.
            BatteryStatus::Available { charge_state, level } => {
                let battery_percent = self.get_battery_percent(val);
                if level != Some(battery_percent) {
                    self.commit(BatteryStatus::Available {
                        charge_state,
                        level: Some(battery_percent),
                    });
                }
            }
            // First ADC reading: transition from Unavailable.
            BatteryStatus::Unavailable => {
                let battery_percent = self.get_battery_percent(val);
                self.commit(BatteryStatus::Available {
                    charge_state: ChargeState::Unknown,
                    level: Some(battery_percent),
                });
            }
        }
    }

    async fn on_charging_state_event(&mut self, event: ChargingStateEvent) {
        let charging = event.charging;
        info!("Charging state changed: {:?}", charging);

        #[cfg(feature = "_ble")]
        {
            let status = if charging {
                // Keep current level when charging
                let level = match self.battery_status {
                    BatteryStatus::Available { level, .. } => level,
                    BatteryStatus::Unavailable => None,
                };
                BatteryStatus::Available {
                    charge_state: ChargeState::Charging,
                    level,
                }
            } else {
                // When unplugged, mark the level unknown and mark status as discharging
                BatteryStatus::Available {
                    charge_state: ChargeState::Discharging,
                    level: None,
                }
            };

            self.commit(status);
        }
    }
}
