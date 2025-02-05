#![deny(warnings, rust_2018_idioms)]
#![cfg(feature = "broken")]

mod support;
use crate::support::*;

use futures::Stream;
use tokio_timer::*;

#[test]
#[should_panic]
fn interval_zero_duration() {
    mocked(|_, time| {
        let _ = Interval::new(time.now(), ms(0));
    });
}

#[test]
fn usage() {
    mocked(|timer, time| {
        let start = time.now();
        let mut int = Interval::new(start, ms(300));

        assert_ready_eq!(int, Some(start));
        assert_not_ready!(int);

        advance(timer, ms(100));
        assert_not_ready!(int);

        advance(timer, ms(200));
        assert_ready_eq!(int, Some(start + ms(300)));
        assert_not_ready!(int);

        advance(timer, ms(400));
        assert_ready_eq!(int, Some(start + ms(600)));
        assert_not_ready!(int);

        advance(timer, ms(500));
        assert_ready_eq!(int, Some(start + ms(900)));
        assert_ready_eq!(int, Some(start + ms(1200)));
        assert_not_ready!(int);
    });
}
