#[cfg(not(test))]
use embassy_nrf::saadc::Saadc;
use embassy_time::{Duration, Instant};
use rmk_macro::{Event, input_device};
#[cfg(test)]
use tests::Saadc;

use super::AnalogEventType;
use crate::event::{Axis, AxisEvent, AxisValType, BatteryAdcEvent, PointingEvent};

/// Events produced by NrfAdc.
#[derive(Event, Clone, Debug)]
pub enum NrfAdcEvent {
    Pointing(PointingEvent),
    Battery(BatteryAdcEvent),
}

#[input_device(publish = NrfAdcEvent)]
pub struct NrfAdc<'a, const PIN_NUM: usize, const EVENT_NUM: usize> {
    saadc: Saadc<'a, PIN_NUM>,
    polling_interval: Duration,
    light_sleep: Option<Duration>,
    sample: [i16; PIN_NUM],
    previous_sample: [i16; PIN_NUM],
    event_type: [AnalogEventType; EVENT_NUM],
    /// Device id emitted in PointingEvent for each event slot.
    /// Indexed by event_state; irrelevant for Battery slots (use 0).
    event_device_ids: [u8; EVENT_NUM],
    event_state: u8,
    channel_state: u8,
    last_activity: Option<Instant>,
    has_sample: bool,
    next_battery_report: Instant,
    battery_report_interval: Duration,
}

impl<'a, const PIN_NUM: usize, const EVENT_NUM: usize> NrfAdc<'a, PIN_NUM, EVENT_NUM> {
    /// Battery channels require Embassy's default SAADC configuration.
    pub fn new(
        saadc: Saadc<'a, PIN_NUM>,
        event_type: [AnalogEventType; EVENT_NUM],
        event_device_ids: [u8; EVENT_NUM],
        polling_interval: Duration,
        light_sleep: Option<Duration>,
    ) -> Self {
        let battery_report_interval = if event_type
            .iter()
            .any(|event| matches!(event, AnalogEventType::Joystick(_)))
        {
            Duration::from_secs(30)
        } else {
            polling_interval
        };
        Self {
            saadc,
            polling_interval,
            event_type,
            event_device_ids,
            light_sleep,
            sample: [0; PIN_NUM],
            previous_sample: [0; PIN_NUM],
            event_state: EVENT_NUM as u8,
            channel_state: 0,
            last_activity: None,
            has_sample: false,
            next_battery_report: Instant::MIN,
            battery_report_interval,
        }
    }
}

impl<'a, const PIN_NUM: usize, const EVENT_NUM: usize> NrfAdc<'a, PIN_NUM, EVENT_NUM> {
    async fn read_nrf_adc_event(&mut self) -> NrfAdcEvent {
        loop {
            // Sample once, then return every event from that scan before waiting again.
            if self.event_state == EVENT_NUM as u8 {
                if self.has_sample {
                    let interval = if self
                        .last_activity
                        .is_some_and(|at| at.elapsed() <= Duration::from_millis(1200))
                    {
                        self.polling_interval
                    } else {
                        self.light_sleep.unwrap_or(self.polling_interval)
                    };
                    embassy_time::Timer::after(interval).await;
                    if self.channel_state != PIN_NUM as u8 {
                        error!("ADC channel count does not match the configured events");
                    }
                    self.previous_sample = self.sample;
                }
                self.saadc.sample(&mut self.sample).await;
                if !self.has_sample {
                    self.previous_sample = self.sample;
                    self.has_sample = true;
                }
                self.channel_state = 0;
                self.event_state = 0;
            }

            match self.event_type[self.event_state as usize] {
                AnalogEventType::Joystick(sz) => {
                    let mut e = [
                        AxisEvent {
                            typ: AxisValType::Rel,
                            axis: Axis::X,
                            value: 0,
                        },
                        AxisEvent {
                            typ: AxisValType::Rel,
                            axis: Axis::Y,
                            value: 0,
                        },
                        AxisEvent {
                            typ: AxisValType::Rel,
                            axis: Axis::Z,
                            value: 0,
                        },
                    ];
                    if sz > 3 || sz == 0 {
                        error!("Joystick with more than 3 dimensions or empty is not supported. Skip this event");
                        self.event_state += 1;
                        continue;
                    }
                    for axis in &mut e[..usize::from(sz)] {
                        let channel = self.channel_state as usize;
                        let value = self.sample[channel];
                        if (i32::from(value) - i32::from(self.previous_sample[channel])).abs() > 150 {
                            self.last_activity = Some(Instant::now());
                        }
                        axis.value = (value + i16::MIN / 2).saturating_mul(2);
                        self.channel_state += 1;
                    }
                    let device_id = self.event_device_ids[self.event_state as usize];
                    self.event_state += 1;
                    return NrfAdcEvent::Pointing(PointingEvent { device_id, axes: e });
                }
                AnalogEventType::Battery => {
                    // Convert to millivolts using Embassy's default SAADC settings.
                    let battery_adc_value =
                        (u32::from(self.sample[self.channel_state as usize].max(0) as u16) * 3600 / 4096) as u16;
                    self.channel_state += 1;
                    self.event_state += 1;
                    let now = Instant::now();
                    if now < self.next_battery_report {
                        continue;
                    }
                    self.next_battery_report = now + self.battery_report_interval;
                    return NrfAdcEvent::Battery(BatteryAdcEvent(battery_adc_value));
                }
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    // Substitute only the hardware sample operation; exercise the production scan loop.
    pub struct Saadc<'a, const N: usize> {
        pub samples: &'a mut VecDeque<[i16; N]>,
    }

    impl<const N: usize> Saadc<'_, N> {
        pub async fn sample(&mut self, output: &mut [i16; N]) {
            *output = self.samples.pop_front().expect("unexpected extra ADC scan");
        }
    }

    #[test]
    fn shared_scans_keep_joystick_activity_and_battery_reporting_independent() {
        crate::test_support::test_block_on(async {
            let mut samples = VecDeque::from([[1000, 1000, 1000], [3000, 1000, 1000]]);
            samples.extend([[3000, 1000, 1300]; 64]);
            let mut adc = NrfAdc::new(
                Saadc { samples: &mut samples },
                [AnalogEventType::Battery, AnalogEventType::Joystick(2)],
                [0, 7],
                Duration::from_millis(20),
                Some(Duration::from_millis(350)),
            );
            let first = Instant::now();
            assert!(matches!(adc.read_nrf_adc_event().await, NrfAdcEvent::Battery(_)));
            let NrfAdcEvent::Pointing(event) = adc.read_nrf_adc_event().await else {
                panic!("joystick event")
            };
            assert_eq!(event.device_id, 7);
            assert_eq!(event.axes[0].value, -30768);
            assert_eq!(event.axes[1].value, -30768);
            assert_eq!(Instant::now(), first);

            // Battery-only change stays idle; movement on Y enables 20 ms scans,
            // then an unchanged joystick returns to 350 ms scans after 1.2 s.
            for ms in [350, 700].into_iter().chain((720..=1920).step_by(20)).chain([2270]) {
                assert!(matches!(adc.read_nrf_adc_event().await, NrfAdcEvent::Pointing(_)));
                assert_eq!(Instant::now() - first, Duration::from_millis(ms));
            }
            embassy_time::Timer::at(first + Duration::from_millis(29650)).await;
            let NrfAdcEvent::Battery(BatteryAdcEvent(millivolts)) = adc.read_nrf_adc_event().await else {
                panic!("battery report due at 30 seconds")
            };
            assert_eq!(millivolts, 2636);
            assert_eq!(Instant::now() - first, Duration::from_secs(30));
            let NrfAdcEvent::Pointing(event) = adc.read_nrf_adc_event().await else {
                panic!("joystick event from the same scan")
            };
            assert_eq!(event.axes[1].value, -30168);
            assert_eq!(Instant::now() - first, Duration::from_secs(30));
            assert!(adc.saadc.samples.is_empty());
        });
    }

    #[test]
    fn battery_only_uses_the_configured_polling_interval() {
        crate::test_support::test_block_on(async {
            let mut samples = VecDeque::from([[1000], [2000], [3000]]);
            let mut adc = NrfAdc::new(
                Saadc { samples: &mut samples },
                [AnalogEventType::Battery],
                [0],
                Duration::from_secs(2),
                None,
            );
            let first = Instant::now();
            for (seconds, expected) in [(0, 878), (2, 1757), (4, 2636)] {
                let NrfAdcEvent::Battery(BatteryAdcEvent(millivolts)) = adc.read_nrf_adc_event().await else {
                    panic!("battery event")
                };
                assert_eq!(millivolts, expected);
                assert_eq!(Instant::now() - first, Duration::from_secs(seconds));
            }
        });
    }
}
