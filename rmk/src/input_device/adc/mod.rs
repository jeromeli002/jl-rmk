// embassy-nrf compiles `saadc` out for parts that lack it (nRF52820, nRF5340-net,
// nRF51), so this module can't build on `_no_saadc` chips.
#[cfg(all(feature = "_nrf_ble", not(feature = "_no_saadc")))]
pub mod nrf;

#[cfg(all(feature = "_nrf_ble", not(feature = "_no_saadc")))]
pub use nrf::*;

pub enum AnalogEventType {
    Joystick(u8),
    Battery,
}

#[derive(PartialEq)]
pub enum AdcState {
    Active,
    LightSleep,
    // DeepSleep,
}

/// Polls a chip-specific ADC reader returning millivolts at the input pin.
/// A failed sample returns `None` and is retried after the polling interval.
#[rmk_macro::input_device(publish = crate::event::BatteryAdcEvent)]
pub struct BatteryAdc<F: core::ops::AsyncFnMut() -> Option<u16>> {
    read_mv: F,
    interval: embassy_time::Duration,
    sampled: bool,
}

impl<F: core::ops::AsyncFnMut() -> Option<u16>> BatteryAdc<F> {
    pub fn new(read_mv: F, interval: embassy_time::Duration) -> Self {
        assert!(interval.as_ticks() > 0, "battery ADC interval must be nonzero");
        Self {
            read_mv,
            interval,
            sampled: false,
        }
    }

    async fn read_battery_adc_event(&mut self) -> crate::event::BatteryAdcEvent {
        loop {
            if self.sampled {
                embassy_time::Timer::after(self.interval).await;
            }
            self.sampled = true;
            if let Some(mv) = (self.read_mv)().await {
                return crate::event::BatteryAdcEvent(mv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BatteryAdc;
    use crate::core_traits::Runnable;
    use crate::event::{BatteryAdcEvent, SubscribableEvent};

    #[test]
    fn battery_reader_publishes_millivolts_from_successive_samples() {
        crate::test_support::test_block_on(async {
            let mut samples = [Some(1300), None, Some(1400)].into_iter();
            let mut adc = BatteryAdc::new(
                async move || samples.next().flatten(),
                embassy_time::Duration::from_millis(1),
            );
            let mut subscriber = BatteryAdcEvent::subscriber();
            let started = embassy_time::Instant::now();
            embassy_futures::select::select(adc.run(), async {
                assert_eq!(subscriber.next_message_pure().await.0, 1300);
                assert_eq!(embassy_time::Instant::now(), started);
                assert_eq!(subscriber.next_message_pure().await.0, 1400);
                assert_eq!(started.elapsed(), embassy_time::Duration::from_millis(2));
            })
            .await;
        });
    }
}
