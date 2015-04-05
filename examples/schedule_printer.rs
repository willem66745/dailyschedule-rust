extern crate dailyschedule;
extern crate time;

use dailyschedule::*;
use time::{Timespec, at_utc, Duration};

const ON: ScheduleContext = ScheduleContext(0);
const OFF: ScheduleContext = ScheduleContext(1);

struct PrintAction {
    id: String
}

impl PrintAction {
    fn new(name: &str) -> PrintAction {
        PrintAction {
            id: name.to_string()
        }
    }
}

impl ScheduleAction for PrintAction {
    fn kick(&self, timestamp: &Timespec, event: &ScheduleMoment, context: &ScheduleContext) {
        let action = match context {
            &ON => "on:",
            &OFF => "off:",
            _ => unreachable!()
        };
        println!("{} {:5}{} {}", self.id, action, at_utc(*timestamp).rfc822(), event);
    }
}

fn main() {
    let action_handler_1 = PrintAction::new("1");
    let action_handler_2 = PrintAction::new("2");
    let action_handler_3 = PrintAction::new("3");
    let action_handler_4 = PrintAction::new("4");

    let mut schedule = Schedule::new().unwrap();

    schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(10,0,0),
                                             ScheduleTime::new(11,0,0)), &action_handler_1, ON);
    schedule.add_event(ScheduleMoment::Fuzzy(ScheduleTime::new(12,0,0),
                                             ScheduleTime::new(13,0,0)), &action_handler_1, OFF);
    schedule.add_event(ScheduleMoment::ByClosure(Box::new(|_| ScheduleTime::new(14,0,0)), Duration::minutes(1)), &action_handler_2, ON);
    schedule.add_event(ScheduleMoment::ByClosure(Box::new(|_| ScheduleTime::new(15,0,0)), Duration::minutes(1)), &action_handler_2, OFF);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(19,0,0)), &action_handler_3, ON);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(21,0,0)), &action_handler_3, OFF);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(22,0,0)), &action_handler_4, ON);
    schedule.add_event(ScheduleMoment::Fixed(ScheduleTime::new(23,0,0)), &action_handler_4, OFF);

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
