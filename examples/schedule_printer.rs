extern crate dailyschedule;
extern crate time;

use dailyschedule::*;
use time::{Timespec, at_utc};

struct PrintOnAction;

impl ScheduleAction for PrintOnAction {
    fn kick(&self, timestamp: &Timespec, event: &ScheduleMoment) {
        println!("on:  {} {:?}", at_utc(*timestamp).rfc822(), event);
    }
}

struct PrintOffAction;

impl ScheduleAction for PrintOffAction {
    fn kick(&self, timestamp: &Timespec, event: &ScheduleMoment) {
        println!("off: {} {:?}", at_utc(*timestamp).rfc822(), event);
    }
}

fn main() {
    let action_on_handler = PrintOnAction;
    let action_off_handler = PrintOffAction;

    let mut schedule = Schedule::new();

    schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(10,00),
                                             ScheduleTime::new(11,00)), &action_on_handler);
    schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(12,00),
                                             ScheduleTime::new(13,00)), &action_off_handler);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(19,00)), &action_on_handler);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(21,00)), &action_off_handler);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(22,00)), &action_on_handler);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(23,00)), &action_off_handler);

    schedule.update_schedule(Timespec{sec:0, nsec:0});
    schedule.update_schedule(Timespec{sec:24*3600, nsec:0});

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
