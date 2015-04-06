extern crate dailyschedule;
extern crate daylight;
extern crate time;

use dailyschedule::*;
use time::{Timespec, at, at_utc, now_utc, Duration};
use daylight::calculate_daylight;
use std::cell::RefCell;

const ON: ScheduleContext = ScheduleContext(0);
const OFF: ScheduleContext = ScheduleContext(1);
const LAT: f64 = 52.0 + 13.0/60.0;
const LONG: f64 = 5.0 + 58.0/60.0;

#[derive(Eq, PartialEq)]
enum SwitchState {
    Off,
    On
}
enum SwitchScheduleState {
    DeepOff,
    Off,
    On
}

struct PrintAction {
    id: String,
    switch_depth: SwitchScheduleState,
    cur_state: SwitchState
}

impl PrintAction {
    fn new(name: &str) -> PrintAction {
        PrintAction {
            id: name.to_string(),
            switch_depth: SwitchScheduleState::Off,
            cur_state: SwitchState::Off
        }
    }

    fn as_box(name: &str) -> Box<dailyschedule::ScheduleAction> {
        Box::new(PrintAction::new(name))
    }
}

impl ScheduleAction for PrintAction {
    fn kick(&mut self, timestamp: &Timespec, event: &ScheduleMoment, context: &ScheduleContext) {
        self.switch_depth = match context {
            &ON => match self.switch_depth {
                SwitchScheduleState::DeepOff => SwitchScheduleState::Off,
                SwitchScheduleState::Off => SwitchScheduleState::On,
                SwitchScheduleState::On => SwitchScheduleState::On
            },
            &OFF => match self.switch_depth {
                SwitchScheduleState::DeepOff => SwitchScheduleState::DeepOff,
                SwitchScheduleState::Off => SwitchScheduleState::DeepOff,
                SwitchScheduleState::On => SwitchScheduleState::Off
            },
            _ => unreachable!()
        };
        let new_state = match self.switch_depth {
            SwitchScheduleState::DeepOff | SwitchScheduleState::Off => SwitchState::Off,
            SwitchScheduleState::On => SwitchState::On
        };
        if new_state != self.cur_state {
            let action = match new_state {
                SwitchState::Off => "off:",
                SwitchState::On => "on:"
            };
            println!("{} {:5}{} {}", self.id, action, at(*timestamp).rfc822(), event);
            self.cur_state = new_state;
        }
    }
}

fn main() {
    //let action_handler_1 = RefCell::new(Box::new(PrintAction::new("1")) as Box<dailyschedule::ScheduleAction>);
    let action_handler_1 = RefCell::new(PrintAction::as_box("1"));
    //let action_handler_2 = PrintAction::new("2");
    //let action_handler_3 = PrintAction::new("3");
    //let action_handler_4 = PrintAction::new("4");

    let mut schedule = Schedule::new().unwrap();

    schedule.add_event(
        ScheduleMoment::Fuzzy(ScheduleTime::new(6,20,0), ScheduleTime::new(6,40,1)),
        &action_handler_1,
        ON);
    schedule.add_event(
        ScheduleMoment::ByClosure(Box::new(|ts| ScheduleTime::new_from_timespec(
                    calculate_daylight(at_utc(ts), LAT, LONG).sunrise)),
                    Duration::minutes(2)),
        &action_handler_1,
        OFF);

    //schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(10,0,0),
    //                                         ScheduleTime::new(11,0,0)), &action_handler_1, ON);
    //schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(12,0,0),
    //                                         ScheduleTime::new(13,0,0)), &action_handler_1, OFF);
    //schedule.add_event(ScheduleMoment::ByClosure(Box::new(|_| ScheduleTime::new(14,0,0)), Duration::minutes(1)), &action_handler_2, ON);
    //schedule.add_event(ScheduleMoment::ByClosure(Box::new(|_| ScheduleTime::new(15,0,0)), Duration::minutes(1)), &action_handler_2, OFF);
    //schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(19,0,0)), &action_handler_3, ON);
    //schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(21,0,0)), &action_handler_3, OFF);
    //schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(22,0,0)), &action_handler_4, ON);
    //schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(23,0,0)), &action_handler_4, OFF);

    let mut tm = now_utc();
    tm.tm_hour = 0;
    tm.tm_min = 0;
    tm.tm_sec = 0;
    tm.tm_nsec = 0;
    let ts_ref = tm.to_timespec();

    for days in 0..366 {
        schedule.update_schedule(ts_ref + Duration::days(days));
    }

    let mut now = Timespec::new(0,0);

    loop {
        match schedule.kick_event(now) {
            Some(next) => {
                now = next;
            }
            None => break
        }
    }

    //schedule.print_keys();
    //schedule.kick_event(Timespec::new(15*3600, 0));
    //schedule.print_keys();
}
