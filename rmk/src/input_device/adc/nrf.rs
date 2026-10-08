#[cfg(not(test))]
use embassy_nrf::saadc::Saadc;
use embassy_time::{Duration, Instant};
use rmk_macro::{Event, input_device};
#[cfg(test)]
use tests::Saadc;

use super::{AdcState, AnalogEventType};
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
    buf: [[i16; PIN_NUM]; 2],
    event_type: [AnalogEventType; EVENT_NUM],
    /// Device id emitted in PointingEvent for each event slot.
    /// Indexed by event_state; irrelevant for Battery slots (use 0).
    event_device_ids: [u8; EVENT_NUM],
    event_state: u8,
    channel_state: u8,
    buf_state: bool,
    adc_state: AdcState,
    active_instant: Instant,
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
            buf: [[0; PIN_NUM]; 2],
            event_state: EVENT_NUM as u8,
            channel_state: 0,
            buf_state: false,
            adc_state: AdcState::LightSleep,
            active_instant: Instant::MIN,
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
                    let interval = if self.adc_state == AdcState::LightSleep {
                        self.light_sleep.unwrap_or(self.polling_interval)
                    } else {
                        self.polling_interval
                    };
                    embassy_time::Timer::after(interval).await;
                    if self.channel_state != PIN_NUM as u8 {
                        error!("ADC channel count does not match the configured events");
                    }
                }
                if self.active_instant.elapsed().as_millis() > 1200 {
                    self.adc_state = AdcState::LightSleep;
                }
                self.buf_state = !self.buf_state;
                let buf = if self.buf_state {
                    &mut self.buf[0]
                } else {
                    &mut self.buf[1]
                };
                self.saadc.sample(buf).await;
                let mut channel = 0;
                for event in &self.event_type {
                    match event {
                        AnalogEventType::Battery => channel += 1,
                        AnalogEventType::Joystick(axes) => {
                            let end = channel + usize::from(*axes);
                            if self.has_sample
                                && self.buf[0][channel..end]
                                    .iter()
                                    .zip(&self.buf[1][channel..end])
                                    .any(|(a, b)| (i32::from(*a) - i32::from(*b)).abs() > 150)
                            {
                                self.adc_state = AdcState::Active;
                                self.active_instant = Instant::now();
                            }
                            channel = end;
                        }
                    }
                }
                self.has_sample = true;
                self.channel_state = 0;
                self.event_state = 0;
            }

            let buf = if self.buf_state { &self.buf[0] } else { &self.buf[1] };

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
                    } else {
                        for i in 0..sz {
                            e[i as usize].value = (buf[self.channel_state as usize] + i16::MIN / 2).saturating_mul(2);
                            self.channel_state += 1;
                        }
                    }
                    let device_id = self.event_device_ids[self.event_state as usize];
                    self.event_state += 1;
                    return NrfAdcEvent::Pointing(PointingEvent { device_id, axes: e });
                }
                AnalogEventType::Battery => {
                    // Convert to millivolts using Embassy's default SAADC settings.
                    let battery_adc_value =
                        (u32::from(buf[self.channel_state as usize].max(0) as u16) * 3600 / 4096) as u16;
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
    fn scan_outputs_share_a_timestamp_and_battery_changes_do_not_wake_joystick() {
        crate::test_support::test_block_on(async {
            let mut samples = VecDeque::from([[1000, 1000], [3000, 1000], [3000, 1300]]);
            let mut adc = NrfAdc::new(
                Saadc { samples: &mut samples },
                [AnalogEventType::Battery, AnalogEventType::Joystick(1)],
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
            assert_eq!(Instant::now(), first);

            assert!(matches!(adc.read_nrf_adc_event().await, NrfAdcEvent::Pointing(_)));
            assert_eq!(Instant::now() - first, Duration::from_millis(350));
            assert!(adc.adc_state == AdcState::LightSleep);
            assert!(matches!(adc.read_nrf_adc_event().await, NrfAdcEvent::Pointing(_)));
            assert!(adc.adc_state == AdcState::Active);
        });
    }
}
