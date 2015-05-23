extern crate dailyschedule;
extern crate daylight;
extern crate time;

use dailyschedule::*;
use time::{Timespec, at_utc, now_utc, Duration};
use daylight::calculate_daylight;
use std::cell::Cell;
use std::rc::Rc;

const ON: Context = Context(0); // always switch on
const ON_WEAK: Context = Context(1); // switch on, except when 2x OFF (intended for dynamic end times)
const OFF: Context = Context(2); // always switch off (2x means that the next ON_WEAK will have no effect)
const OFF_WEAK: Context = Context(3); // always swich off
const LAT: f64 = 52.0 + 13.0/60.0;
const LONG: f64 = 5.0 + 58.0/60.0;

#[derive(Copy, Clone, Eq, PartialEq)]
enum SwitchState {
    Off,
    On
}

// FIXME: Not a reliable approach
#[derive(Copy, Clone)]
enum SwitchScheduleState {
    DeepOff,
    Off,
    On
}

struct PrintAction {
    id: String,
    switch_depth: Cell<SwitchScheduleState>,
    cur_state: Cell<SwitchState>
}

impl PrintAction {
    fn new(name: &str) -> PrintAction {
        PrintAction {
            id: name.to_string(),
            switch_depth: Cell::new(SwitchScheduleState::Off),
            cur_state: Cell::new(SwitchState::Off)
        }
    }

    fn as_ref(name: &str) -> Rc<PrintAction> {
        Rc::new(PrintAction::new(name))
    }
}

impl Handler for PrintAction {
    fn kick(&self, timestamp: &Timespec, event: &DailyEvent, context: &Context) {
        self.switch_depth.set(match context {
            &ON => match self.switch_depth.get() {
                SwitchScheduleState::DeepOff => SwitchScheduleState::On,
                SwitchScheduleState::Off => SwitchScheduleState::On,
                SwitchScheduleState::On => SwitchScheduleState::On
            },
            &ON_WEAK => match self.switch_depth.get() {
                SwitchScheduleState::DeepOff => SwitchScheduleState::Off,
                SwitchScheduleState::Off => SwitchScheduleState::On,
                SwitchScheduleState::On => SwitchScheduleState::On
            },
            &OFF => match self.switch_depth.get() {
                SwitchScheduleState::DeepOff => SwitchScheduleState::DeepOff,
                SwitchScheduleState::Off => SwitchScheduleState::DeepOff,
                SwitchScheduleState::On => SwitchScheduleState::Off
            },
            &OFF_WEAK => match self.switch_depth.get() {
                SwitchScheduleState::DeepOff => SwitchScheduleState::DeepOff,
                SwitchScheduleState::Off => SwitchScheduleState::Off,
                SwitchScheduleState::On => SwitchScheduleState::Off
            },
            _ => unreachable!()
        });
        let new_state = match self.switch_depth.get() {
            SwitchScheduleState::DeepOff | SwitchScheduleState::Off => SwitchState::Off,
            SwitchScheduleState::On => SwitchState::On
        };
        if new_state != self.cur_state.get() {
            let action = match new_state {
                SwitchState::Off => "off:",
                SwitchState::On => "on:"
            };
            println!("{} {:5}{} {:?}", self.id, action, at_utc(*timestamp).rfc822(), event);
            self.cur_state.set(new_state);
        }
    }
}

fn main() {
    let sunrise_closure = Box::new(|ts| Moment::new_from_timespec(calculate_daylight(at_utc(ts), LAT, LONG).sunrise));
    let sunset_closure = Box::new(|ts| Moment::new_from_timespec(calculate_daylight(at_utc(ts), LAT, LONG).sunset));

    let action_handler_1 = PrintAction::as_ref("1");
    let action_handler_2 = PrintAction::as_ref("2");

    let mut schedule = Schedule::<PrintAction>::new_local().unwrap();

    schedule.add_event(
        DailyEvent::Fuzzy(Filter::MonToFri, Moment::new(6,20,0), Moment::new(6,40,0)),
        action_handler_1.clone(),
        ON_WEAK);
    schedule.add_event(
        DailyEvent::ByClosure(Filter::MonToFri, sunrise_closure, Duration::minutes(2)),
        action_handler_1.clone(),
        OFF);

    schedule.add_event(
        DailyEvent::ByClosure(Filter::Always, sunset_closure, Duration::minutes(10)),
        action_handler_2.clone(),
        ON);
    schedule.add_event(
        DailyEvent::Fuzzy(Filter::Always, Moment::new(0,15,0), Moment::new(0,30,0)),
        action_handler_2.clone(),
        OFF_WEAK);

    let mut tm = now_utc();
    tm.tm_hour = 0;
    tm.tm_min = 0;
    tm.tm_sec = 0;
    tm.tm_nsec = 0;
    let ts_ref = tm.to_timespec();

    for days in 0..730 {
        schedule.update_schedule(ts_ref + Duration::days(days));
    }

    let mut now = now_utc().to_timespec();

    loop {
        match schedule.kick_event(now) {
            Some(next) => {
                now = next;
            }
            None => break
        }
    }
}
