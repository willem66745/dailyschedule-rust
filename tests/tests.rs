#![deny(warnings)]
extern crate dailyschedule;
extern crate time;
extern crate zoneinfo;

use dailyschedule::*;
use std::cell::RefCell;
use zoneinfo::ZoneInfo;

const DUMMY: Context = Context(0);
const ONE: Context = Context(1);
const TWO: Context = Context(2);

struct TestHandler {
    timestamps: Vec<time::Timespec>,
    contexts: Vec<Context>
}

impl TestHandler {
    fn new() -> TestHandler {
        TestHandler {
            timestamps: vec![],
            contexts: vec![]
        }
    }

    fn as_ref() -> RefCell<TestHandler> {
        RefCell::new(TestHandler::new())
    }
}

impl Handler for TestHandler {
    fn kick(&mut self, timestamp: &time::Timespec, _: &DailyEvent, context: &Context) {
        self.timestamps.push((*timestamp).clone());
        self.contexts.push(*context);
    }
}

#[test]
fn fixed_one_day_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        &handler,
        DUMMY);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert_eq!(next_event, time::Timespec::new(7200, 0)); // 1970-1-1 2:00

    let next_event = schedule.kick_event(next_event);

    assert_eq!(next_event, None);

    let next_event = schedule.peek_event();

    assert_eq!(next_event, None);

    // handler must have captured 1 timestamp
    let timestamps = &handler.borrow().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), time::Timespec::new(7200, 0));
}

#[test]
fn fuzzy_one_day_nodst() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fuzzy(Filter::Always, Moment::new(2,0,0), Moment::new(3,0,0)),
        &handler,
        DUMMY);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert!(next_event >= time::Timespec::new(7200, 0)); // 1970-1-1 2:00
    assert!(next_event <= time::Timespec::new(10800, 0)); // 1970-1-1 3:00

    let next_event_none = schedule.kick_event(next_event);

    assert_eq!(next_event_none, None);

    let next_event_none = schedule.peek_event();

    assert_eq!(next_event_none, None);

    let timestamps = &handler.borrow().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), next_event);
}

#[test]
fn byclosure_one_day_nodst() {
    let closure = |_| Moment::new(2,0,0);
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, &closure, time::Duration::seconds(0)),
        &handler,
        DUMMY);
    schedule.update_schedule(time::Timespec::new(0, 0));

    let next_event = schedule.peek_event().unwrap();

    assert_eq!(next_event, time::Timespec::new(7200, 0)); // 1970-1-1 2:00

    let next_event_none = schedule.kick_event(next_event);

    assert_eq!(next_event_none, None);

    let next_event_none = schedule.peek_event();

    assert_eq!(next_event_none, None);

    let timestamps = &handler.borrow().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert_eq!(*timestamps.iter().nth(0).unwrap(), next_event);
}

#[test]
fn contexts() {
    let zoneinfo = ZoneInfo::by_tz("UTC").unwrap();
    let handler = TestHandler::as_ref();
    let mut schedule = Schedule::<TestHandler>::new(zoneinfo);

    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(2,0,0)),
        &handler,
        ONE);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(3,0,0)),
        &handler,
        TWO);
    schedule.add_event(
        DailyEvent::Fixed(Filter::Always, Moment::new(4,0,0)),
        &handler,
        ONE);

    let ref_time = time::Timespec::new(0, 0);

    // schedule events for 3 days
    for days in (0..3) {
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
    assert_eq!(handler.borrow().contexts.iter().cloned().collect::<Vec<Context>>(),
               [ONE, TWO, ONE, ONE, TWO, ONE, ONE, TWO, ONE]);

    // check the handler whather all expected timestamps has been passed
    assert_eq!(handler.borrow().timestamps.iter().cloned().collect::<Vec<time::Timespec>>(),
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
