#![deny(warnings)]
extern crate dailyschedule;
extern crate time;
extern crate zoneinfo;

use dailyschedule::*;
use std::cell::RefCell;
use std::rc::Rc;
use zoneinfo::ZoneInfo;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Context {
    Dummy,
    One,
    Two
}

struct TestHandler {
    hints: RefCell<Vec<time::Timespec>>,
    timestamps: RefCell<Vec<time::Timespec>>,
    contexts: RefCell<Vec<Context>>
}

impl TestHandler {
    fn new() -> TestHandler {
        TestHandler {
            hints: RefCell::new(vec![]),
            timestamps: RefCell::new(vec![]),
            contexts: RefCell::new(vec![])
        }
    }

    fn as_ref() -> Rc<TestHandler> {
        Rc::new(TestHandler::new())
    }
}

impl Handler<Context> for TestHandler {
    fn hint(&self, timestamp: &time::Timespec, _: &Context) {
        self.hints.borrow_mut().push((*timestamp).clone());
    }

    fn kick(&self, timestamp: &time::Timespec, context: &Context) {
        assert!(self.hints.borrow().contains(timestamp));
        self.timestamps.borrow_mut().push((*timestamp).clone());
        self.contexts.borrow_mut().push(*context);
    }
}

#[test]
fn fixed_one_day_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::Dummy);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert_eq!(next_event, time::Timespec::new(7200, 0)); // 1970-1-1 2:00

    let next_event = schedule.kick_event(next_event);

    assert_eq!(next_event, None);

    let next_event = schedule.peek_event();

    assert_eq!(next_event, None);

    // handler must have captured 1 timestamp
    let timestamps = &handler.timestamps.borrow();
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), time::Timespec::new(7200, 0));
}

#[test]
fn fuzzy_one_day_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fuzzy(Filter::Always, Moment::new(2,0,0), Moment::new(3,0,0)),
        handler.clone(),
        Context::Dummy);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert!(next_event >= time::Timespec::new(7200, 0)); // 1970-1-1 2:00
    assert!(next_event <= time::Timespec::new(10800, 0)); // 1970-1-1 3:00

    let next_event_none = schedule.kick_event(next_event);

    assert_eq!(next_event_none, None);

    let next_event_none = schedule.peek_event();

    assert_eq!(next_event_none, None);

    let timestamps = &handler.timestamps.borrow();
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), next_event);
}

#[test]
fn byclosure_one_day_nodst() {
    let closure = Box::new(|_| Moment::new(2,0,0));
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, closure, time::Duration::seconds(0)),
        handler.clone(),
        Context::Dummy);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert_eq!(next_event, time::Timespec::new(7200, 0)); // 1970-1-1 2:00

    let next_event_none = schedule.kick_event(next_event);

    assert_eq!(next_event_none, None);

    let next_event_none = schedule.peek_event();

    assert_eq!(next_event_none, None);

    let timestamps = &handler.timestamps.borrow();
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), next_event);
}

#[test]
fn contexts_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::One);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(3,0,0)),
        handler.clone(),
        Context::Two);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(4,0,0)),
        handler.clone(),
        Context::One);

    let ref_time = time::Timespec::new(0, 0);

    // schedule events for 3 days
    for days in 0..3 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected contexts has been passed
    assert_eq!(handler.contexts.borrow().iter().cloned().collect::<Vec<Context>>(),
               [Context::One, Context::Two, Context::One, Context::One, Context::Two, Context::One, Context::One, Context::Two, Context::One]);

    // check the handler whather all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(2),
                ref_time + time::Duration::hours(3),
                ref_time + time::Duration::hours(4),
                ref_time + time::Duration::hours(2) + time::Duration::days(1),
                ref_time + time::Duration::hours(3) + time::Duration::days(1),
                ref_time + time::Duration::hours(4) + time::Duration::days(1),
                ref_time + time::Duration::hours(2) + time::Duration::days(2),
                ref_time + time::Duration::hours(3) + time::Duration::days(2),
                ref_time + time::Duration::hours(4) + time::Duration::days(2)]);
}

#[test]
fn overlapping_order_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::One);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::Two);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::One);

    let ref_time = time::Timespec::new(0, 0);

    // schedule events for 3 days
    for days in 0..3 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected contexts has been passed
    assert_eq!(handler.contexts.borrow().iter().cloned().collect::<Vec<Context>>(),
               [Context::One, Context::Two, Context::One, Context::One, Context::Two, Context::One, Context::One, Context::Two, Context::One]);

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(2),
                ref_time + time::Duration::hours(2),
                ref_time + time::Duration::hours(2),
                ref_time + time::Duration::hours(2) + time::Duration::days(1),
                ref_time + time::Duration::hours(2) + time::Duration::days(1),
                ref_time + time::Duration::hours(2) + time::Duration::days(1),
                ref_time + time::Duration::hours(2) + time::Duration::days(2),
                ref_time + time::Duration::hours(2) + time::Duration::days(2),
                ref_time + time::Duration::hours(2) + time::Duration::days(2)]);
}

#[test]
fn weekend() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Weekend, Moment::new(2,0,0)),
        handler.clone(),
        Context::Dummy);

    // note: EPOCH was a Thursday
    let ref_time = time::Timespec::new(0, 0);

    // schedule events for 8 days
    for days in 0..8 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(2) + time::Duration::days(2),   // 2 days after Thursday
                ref_time + time::Duration::hours(2) + time::Duration::days(3)]); // 3 days after Thursday
}

#[test]
fn weekdays() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::MonToFri, Moment::new(2,0,0)),
        handler.clone(),
        Context::Dummy);

    // note: EPOCH was a Thursday
    let ref_time = time::Timespec::new(0, 0);

    // schedule events for 8 days
    for days in 0..8 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(2) + time::Duration::days(0),
                ref_time + time::Duration::hours(2) + time::Duration::days(1), // day 2 and day 3
                ref_time + time::Duration::hours(2) + time::Duration::days(4), // is weekend after EPOCH
                ref_time + time::Duration::hours(2) + time::Duration::days(5),
                ref_time + time::Duration::hours(2) + time::Duration::days(6),
                ref_time + time::Duration::hours(2) + time::Duration::days(7)]);
}

#[test]
fn to_dst_no_overlap() {
    let closure = Box::new(|ts| Moment::new_from_timespec(ts + time::Duration::hours(5)));
    let zoneinfo = ZoneInfo::by_tz("Europe/Amsterdam").unwrap(); // Same as CET in 2015
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    // create event based on local time (@ Match 29th 2015 the exact transition moment)
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::Dummy);
    // create event based on UTC (provided by closure)
    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, closure, time::Duration::seconds(0)),
        handler.clone(),
        Context::Dummy);

    // March 27th 2015 (two days before DST transition in EU)
    let ref_time = time::Tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 27, tm_mon: 2, tm_year: 115,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_utcoff: 0, tm_nsec: 0
    };
    let ref_time = ref_time.to_timespec();

    // schedule events for 5 days
    for days in 0..5 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(1) + time::Duration::days(0),
                ref_time + time::Duration::hours(5) + time::Duration::days(0),
                ref_time + time::Duration::hours(1) + time::Duration::days(1),
                ref_time + time::Duration::hours(5) + time::Duration::days(1),
                ref_time + time::Duration::hours(0) + time::Duration::days(2), // <- transition; moment shifts from 1:00 UTC to 0:00 UTC
                ref_time + time::Duration::hours(5) + time::Duration::days(2), // <- scheduled as UTC timestamp; stays at 5:00 UTC
                ref_time + time::Duration::hours(0) + time::Duration::days(3),
                ref_time + time::Duration::hours(5) + time::Duration::days(3),
                ref_time + time::Duration::hours(0) + time::Duration::days(4),
                ref_time + time::Duration::hours(5) + time::Duration::days(4)]);
}

#[test]
fn to_dst_overlap() {
    let closure = Box::new(|ts| Moment::new_from_timespec(ts + time::Duration::hours(0)));
    let zoneinfo = ZoneInfo::by_tz("Europe/Amsterdam").unwrap(); // Same as CET in 2015
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    // create event based on local time (@ Match 29th 2015 the exact transition moment)
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::One);
    // create event based on UTC (provided by closure)
    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, closure, time::Duration::seconds(0)),
        handler.clone(),
        Context::Two);

    // March 27th 2015 (two days before DST transition in EU)
    let ref_time = time::Tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 27, tm_mon: 2, tm_year: 115,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_utcoff: 0, tm_nsec: 0
    };
    let ref_time = ref_time.to_timespec();

    // schedule events for 5 days
    for days in 0..5 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(0) + time::Duration::days(0),
                ref_time + time::Duration::hours(1) + time::Duration::days(0),
                ref_time + time::Duration::hours(0) + time::Duration::days(1),
                ref_time + time::Duration::hours(1) + time::Duration::days(1),
                ref_time + time::Duration::hours(0) + time::Duration::days(2), // <- transition; moment shifts from 1:00 UTC to 0:00 UTC
                ref_time + time::Duration::hours(0) + time::Duration::days(2), // <- scheduled as UTC timestamp; stays at 0:00 UTC
                ref_time + time::Duration::hours(0) + time::Duration::days(3),
                ref_time + time::Duration::hours(0) + time::Duration::days(3),
                ref_time + time::Duration::hours(0) + time::Duration::days(4),
                ref_time + time::Duration::hours(0) + time::Duration::days(4)]);

    // check the handler whether all expected contexts has been passed
    assert_eq!(handler.contexts.borrow().iter().cloned().collect::<Vec<Context>>(),
               [Context::Two,
                Context::One,
                Context::Two,
                Context::One,
                Context::One, // DST active, first event overlaps seconds event -> first event has priority
                Context::Two,
                Context::One,
                Context::Two,
                Context::One,
                Context::Two]);
}

#[test]
fn from_dst_no_overlap() {
    let closure = Box::new(|ts| Moment::new_from_timespec(ts + time::Duration::hours(5)));
    let zoneinfo = ZoneInfo::by_tz("Europe/Amsterdam").unwrap(); // Same as CET in 2015
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<Context, TestHandler>::new(zoneinfo);

    // create event based on local time (@ Match 29th 2015 the exact transition moment)
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        handler.clone(),
        Context::Dummy);
    // create event based on UTC (provided by closure)
    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, closure, time::Duration::seconds(0)),
        handler.clone(),
        Context::Dummy);

    // October 23th 2015 (two days before DST transition in EU)
    let ref_time = time::Tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 23, tm_mon: 9, tm_year: 115,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_utcoff: 0, tm_nsec: 0
    };
    let ref_time = ref_time.to_timespec();

    // schedule events for 5 days
    for days in 0..5 {
        schedule.update_schedule(ref_time + time::Duration::days(days));
    }

    let mut next_event = schedule.peek_event().unwrap();

    // execute all events
    loop {
        match schedule.kick_event(next_event) {
            Some(next) => next_event = next,
            None => break
        }
    }

    // check the handler whether all expected timestamps has been passed
    assert_eq!(handler.timestamps.borrow().iter().cloned().collect::<Vec<time::Timespec>>(),
               [ref_time + time::Duration::hours(0) + time::Duration::days(0),
                ref_time + time::Duration::hours(5) + time::Duration::days(0),
                ref_time + time::Duration::hours(0) + time::Duration::days(1),
                ref_time + time::Duration::hours(5) + time::Duration::days(1),
                ref_time + time::Duration::hours(0) + time::Duration::days(2), // <- transition; moment shifts from 0:00 UTC to 1:00 UTC (though event fires at 0:00)
                ref_time + time::Duration::hours(5) + time::Duration::days(2), // <- scheduled as UTC timestamp; stays at 5:00 UTC
                ref_time + time::Duration::hours(1) + time::Duration::days(3),
                ref_time + time::Duration::hours(5) + time::Duration::days(3),
                ref_time + time::Duration::hours(1) + time::Duration::days(4),
                ref_time + time::Duration::hours(5) + time::Duration::days(4)]);
}
