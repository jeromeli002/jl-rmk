use core::cell::Cell;
use core::convert::Infallible;
use core::pin::pin;

use embassy_time::{Duration, Instant, MockDriver};
use futures::poll;
use rmk_macro::processor;

use super::{DeadlineProcessor, Processor};
use crate::core_traits::Runnable;
use crate::event::EventSubscriber;
use crate::test_support::test_block_on;

#[processor(poll_interval = 100, deadline)]
struct Timed<'a, T: Copy> {
    deadlines: &'a Cell<u32>,
    polls: &'a Cell<u32>,
    due: Option<Instant>,
    _marker: T,
}

impl<T: Copy> DeadlineProcessor for Timed<'_, T> {
    fn deadline(&self) -> Option<Instant> {
        self.due
    }

    async fn on_deadline(&mut self) {
        self.deadlines.set(self.deadlines.get() + 1);
        self.due = None;
    }
}

impl<T: Copy> Timed<'_, T> {
    async fn poll(&mut self) {
        self.polls.set(self.polls.get() + 1);
        self.due = Some(Instant::now() + Duration::from_millis(20));
    }
}

#[test]
fn macro_processor_uses_the_public_deadline_trait_directly() {
    test_block_on(async {
        let deadlines = Cell::new(0);
        let polls = Cell::new(0);
        let at = Instant::from_millis(50);
        let mut processor = Timed {
            deadlines: &deadlines,
            polls: &polls,
            due: Some(at),
            _marker: 1u8,
        };
        assert_eq!(DeadlineProcessor::deadline(&processor), Some(at));
        DeadlineProcessor::on_deadline(&mut processor).await;
        assert_eq!(deadlines.get(), 1);
        assert_eq!(DeadlineProcessor::deadline(&processor), None);
        assert_eq!(polls.get(), 0);
    });
}

#[test]
fn toml_executor_and_run_all_use_the_same_deadline_implementation() {
    test_block_on(async {
        let first_deadlines = Cell::new(0);
        let first_polls = Cell::new(0);
        let second_deadlines = Cell::new(0);
        let second_polls = Cell::new(0);
        let mut first = Timed {
            deadlines: &first_deadlines,
            polls: &first_polls,
            due: Some(Instant::from_millis(50)),
            _marker: 1u8,
        };
        let mut second = Timed {
            deadlines: &second_deadlines,
            polls: &second_polls,
            due: Some(Instant::from_millis(50)),
            _marker: 1u8,
        };
        let mut registered = pin!(Runnable::run(&mut first));
        let mut rust = pin!(crate::run_all!(second));
        assert!(poll!(registered.as_mut()).is_pending());
        assert!(poll!(rust.as_mut()).is_pending());
        for (at, expected_deadlines, expected_polls) in [(50, 1, 0), (100, 1, 1), (120, 2, 1), (200, 2, 2), (220, 3, 2)]
        {
            MockDriver::get().advance(Duration::from_millis(at - Instant::now().as_millis()));
            assert!(poll!(registered.as_mut()).is_pending());
            assert!(poll!(rust.as_mut()).is_pending());
            assert_eq!(
                (first_deadlines.get(), first_polls.get()),
                (expected_deadlines, expected_polls)
            );
            assert_eq!(
                (second_deadlines.get(), second_polls.get()),
                (expected_deadlines, expected_polls)
            );
        }
    });
}

struct Manual<'a> {
    due: Option<Instant>,
    fired: &'a Cell<u32>,
}

impl Processor for Manual<'_> {
    type Event = Infallible;

    fn subscriber() -> impl EventSubscriber<Event = Self::Event> {
        core::future::pending::<Infallible>()
    }

    async fn process(&mut self, event: Infallible) {
        match event {}
    }
}

impl DeadlineProcessor for Manual<'_> {
    fn deadline(&self) -> Option<Instant> {
        self.due
    }

    async fn on_deadline(&mut self) {
        self.fired.set(self.fired.get() + 1);
        self.due = None;
    }
}

impl Runnable for Manual<'_> {
    async fn run(&mut self) -> ! {
        self.deadline_loop().await
    }
}

#[test]
fn manual_processor_keeps_the_existing_deadline_api() {
    test_block_on(async {
        let fired = Cell::new(0);
        let mut processor = Manual {
            due: Some(Instant::now()),
            fired: &fired,
        };
        let mut run = pin!(processor.run());
        assert!(poll!(run.as_mut()).is_pending());
        assert!(poll!(run.as_mut()).is_pending());
        assert_eq!(fired.get(), 1);
    });
}
