extern crate rand;
extern crate time;

use time::{Timespec, Duration, at_utc};
use std::collections::BTreeMap;
use std::rc::Rc;
use rand::{Rng, thread_rng};

/// Represents a fixed moment in a day
#[derive(Debug)]
pub struct ScheduleTime {
    hours: u8,
    minutes: u8,
    seconds: u8,
}

impl ScheduleTime {
    /// Create a moment in a day
    pub fn new(h:u8, m:u8) -> ScheduleTime {
        ScheduleTime {
            hours: h,
            minutes: m,
            seconds: 0
        }
    }

    /// Convert schedule time to actual timestamp
    fn create_timestamp(&self, midnight_reference: Timespec) -> Timespec {
        midnight_reference + Duration::hours(self.hours as i64)
                           + Duration::minutes(self.minutes as i64)
                           + Duration::seconds(self.seconds as i64)
    }
}

/// Represent a (abstract) moment in a day
#[derive(Debug)]
pub enum ScheduleMoment {
    Fixed(ScheduleTime),
    Fuzzy(ScheduleTime, ScheduleTime),
    ByClosure(/*closure*/ Duration) // TODO: implement
}

/// Represents a moment and an specific action in a day
pub struct ScheduleEvent<'a>(pub ScheduleMoment, pub &'a ScheduleAction);

impl <'a>ScheduleEvent<'a> {
    /// Determine timestamp for event
    fn create_timestamp(&self, midnight_reference: Timespec) -> Timespec {
        match self.0 {
            ScheduleMoment::Fixed(ref moment) => moment.create_timestamp(midnight_reference),
            ScheduleMoment::Fuzzy(ref m1, ref m2) => {
                // pick a time between both given moment
                let mut rng = rand::thread_rng();
                let t1 = m1.create_timestamp(midnight_reference);
                let t2 = m2.create_timestamp(midnight_reference);
                let t_start = if t1 >= t2 {t2} else {t1};
                let t_end = if t1 >= t2 {t1} else {t2};
                let duration = t_end - t_start;
                t_start + Duration::seconds(rng.gen_range(0, duration.num_seconds()))
            }
            _ => panic!("not implemented!") // TODO: add by closure
        }
    }
}

pub trait ScheduleAction {
    /// Perform a action (in a day)
    fn kick(&self, timestamp: &Timespec, event: &ScheduleMoment);
}

/// Represents multiple moments in a day
pub struct Schedule<'a> {
    /// List of (abstract) moments in a day
    events: Vec<Rc<ScheduleEvent<'a>>>,

    /// Tree of actual scheduled moments and reference to the abstract moment in a day
    schedule: BTreeMap<Timespec, Rc<ScheduleEvent<'a>>>
}

impl <'a>Schedule<'a> {
    /// Create a (empty) list of moments in a day
    pub fn new() -> Schedule<'a> {
        Schedule {
            events: vec![],
            schedule: BTreeMap::new()
        }
    }

    /// Add a (abstract) moment and action in a day
    pub fn add_event(&mut self, moment: ScheduleMoment, action: &'a ScheduleAction) {
        self.events.push(Rc::new(ScheduleEvent(moment, action)));
    }

    /// Update the schedule for 24 hours
    pub fn update_schedule(&mut self, midnight_reference: Timespec) {
        for event in &self.events {
            let timestamp = event.create_timestamp(midnight_reference);
            self.schedule.insert(timestamp, event.clone());
        }
    }

    /// Consume schedule until now and kick last or current event
    pub fn kick_event(&mut self, now: Timespec) -> Option<Timespec> {
        let past_events: Vec<Timespec> = self.schedule.keys().filter(|&k| *k <= now).cloned().collect();

        // kick the current event...
        if let Some(timestamp) = past_events.last() {
            if let Some(schedule_event) = self.schedule.get(timestamp) {
                schedule_event.1.kick(&timestamp, &schedule_event.0);
            }
        }

        // ...and consume that and prior events
        for past_event in past_events {
            self.schedule.remove(&past_event);
        }

        self.schedule.keys().cloned().nth(0)
    }

    /// Peek for next event
    pub fn peek_event(&self) -> Option<Timespec> {
        self.schedule.keys().cloned().nth(0)
    }

    // TODO remove
    pub fn print_keys(&self) {
        for k in self.schedule.keys() {
            println!("{} {:?}", at_utc(*k).rfc822(), self.schedule.get(k).unwrap().0);
        }
    }
}
