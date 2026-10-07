// embassy-nrf compiles `saadc` out for parts that lack it (nRF52820, nRF5340-net,
// nRF51), so this module can't build on `_no_saadc` chips.
#[cfg(all(feature = "_nrf_ble", not(feature = "_no_saadc")))]
pub mod nrf;

#[cfg(all(feature = "_nrf_ble", not(feature = "_no_saadc")))]
pub use nrf::*;

use crate::core_traits::Runnable;
use crate::event::{BatteryAdcEvent, publish_event_async};

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

/// Periodically reads ADC input millivolts and publishes battery readings.
///
/// The reader supplies the ADC input voltage in millivolts. Configure the
/// voltage divider on the battery processor to calculate the battery voltage.
/// Run this task with `run_all!` or [`Runnable::run`].
pub struct BatteryAdc<F: core::ops::AsyncFnMut() -> Option<u16>> {
    read_mv: F,
    interval: embassy_time::Duration,
}

impl<F: core::ops::AsyncFnMut() -> Option<u16>> BatteryAdc<F> {
    /// Creates a battery sampling task with the given reader and interval.
    ///
    /// `read_mv` must return `Some(input_mv)` for a successful reading or `None`
    /// to skip a failed reading. Do not apply the battery voltage divider ratio
    /// in the reader.
    ///
    /// The task reads immediately when started. After publishing a reading or
    /// skipping a failed attempt, it waits `interval` before reading again.
    ///
    /// # Panics
    ///
    /// Panics if `interval` is zero.
    pub fn new(read_mv: F, interval: embassy_time::Duration) -> Self {
        assert!(interval.as_ticks() > 0, "battery ADC interval must be nonzero");
        Self { read_mv, interval }
    }
}

impl<F: core::ops::AsyncFnMut() -> Option<u16>> Runnable for BatteryAdc<F> {
    async fn run(&mut self) -> ! {
        loop {
            if let Some(mv) = (self.read_mv)().await {
                publish_event_async(BatteryAdcEvent(mv)).await;
            }
            embassy_time::Timer::after(self.interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BatteryAdc;
    use crate::core_traits::Runnable;
    use crate::event::{BatteryAdcEvent, SubscribableEvent};

    #[test]
    fn battery_sampling_restarts_without_waiting_for_the_previous_interval() {
        crate::test_support::test_block_on(async {
            let mut samples = [Some(1300), Some(1400)].into_iter();
            let mut adc = BatteryAdc::new(
                async move || samples.next().flatten(),
                embassy_time::Duration::from_secs(30),
            );
            let mut subscriber = BatteryAdcEvent::subscriber();
            let started = embassy_time::Instant::now();
            for expected in [1300, 1400] {
                embassy_futures::select::select(adc.run(), async {
                    assert_eq!(subscriber.next_message_pure().await.0, expected);
                    assert_eq!(embassy_time::Instant::now(), started);
                })
                .await;
            }
        });
    }

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
