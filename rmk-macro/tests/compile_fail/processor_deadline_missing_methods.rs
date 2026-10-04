extern crate self as embassy_time;

pub use rmk::embassy_time::Instant;
use rmk::event::{EventSubscriber, SubscribableEvent};
use rmk::processor::DeadlineProcessor;
use rmk_macro::processor;

#[derive(Clone)]
struct TickEvent;

impl EventSubscriber for TickEvent {
    type Event = Self;
    async fn next_event(&mut self) -> Self {
        core::future::pending().await
    }
}

impl SubscribableEvent for TickEvent {
    type Subscriber = Self;
    fn subscriber() -> Self {
        Self
    }
}

#[processor(subscribe = [TickEvent], deadline)]
struct MissingDeadline;

impl MissingDeadline {
    async fn on_tick_event(&mut self, _: TickEvent) {}
}

impl DeadlineProcessor for MissingDeadline {
    async fn on_deadline(&mut self) {}
}

#[processor(subscribe = [TickEvent], deadline)]
struct MissingHandler;

impl MissingHandler {
    async fn on_tick_event(&mut self, _: TickEvent) {}
}

impl DeadlineProcessor for MissingHandler {
    fn deadline(&self) -> Option<Instant> {
        None
    }
}

#[processor(subscribe = [TickEvent], deadline)]
struct MissingImplementation;

impl MissingImplementation {
    async fn on_tick_event(&mut self, _: TickEvent) {}

    async fn deadline_loop(&mut self) -> ! {
        core::future::pending().await
    }
}

fn main() {}
