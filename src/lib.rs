extern crate rand;
extern crate time;
extern crate zoneinfo;

use time::{Timespec, Duration, at_utc};
use std::collections::BTreeMap;
use std::rc::Rc;
use rand::{Rng, thread_rng};
use zoneinfo::{ZoneInfo, ZoneInfoElement};
use std::io::Result;

/// Represents a fixed moment in a day
#[derive(Debug)]
pub enum ScheduleTime {
    LocalTime(Duration),
    UtcTime(Duration)
}

// Local time definition
enum LocalTimeState {
    // Zoneinfo state is not loaded yet
    Unknown,
    // Current zoneinfo specifies no future daylight saving time change is expected
    NoChangePending(ZoneInfoElement),
    // Daylight saving time change is pending
    ChangePending(Timespec, // transistion time
                  ZoneInfoElement, // zone information before transisition time
                  ZoneInfoElement) // zone information at and after transition time
}

impl ScheduleTime {
    /// Create a moment in a day
    pub fn new(h:u8, m:u8, s:u8) -> ScheduleTime {
        ScheduleTime::LocalTime(
            Duration::hours(h as i64) +
            Duration::minutes(m as i64) +
            Duration::seconds(s as i64))
    }

    /// Create a moment in a day based on Timespec
    pub fn new_from_timespec(ts: Timespec) -> ScheduleTime {
        let mut tm_utc = at_utc(ts);

        tm_utc.tm_hour = 0;
        tm_utc.tm_min = 0;
        tm_utc.tm_sec = 0;
        tm_utc.tm_nsec = 0;

        ScheduleTime::UtcTime(ts - tm_utc.to_timespec())
    }

    /// Convert schedule time to actual time stamp
    fn create_timestamp(&self, ut_midnight_reference: Timespec,
                        localtime: &LocalTimeState) -> Timespec {
        match self {
            // timestamp is simply a reference to UTC so just add the offset
            &ScheduleTime::UtcTime(offset) => ut_midnight_reference + offset,
            // timestamp is a reference to the moment in a day
            &ScheduleTime::LocalTime(offset) => { 
                let ut_offset = match *localtime {
                    LocalTimeState::NoChangePending(ref info) => info.ut_offset,
                    LocalTimeState::ChangePending(transition_time, ref before, ref after) => {
                        if ut_midnight_reference < transition_time {
                            before.ut_offset
                        }
                        else {
                            after.ut_offset
                        }
                    }
                    _ => unreachable!()
                };

                ut_midnight_reference + offset + ut_offset
            }
        }
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
    fn create_timestamp(&self, ut_midnight_reference: Timespec,
                        localtime: &LocalTimeState) -> Timespec {
        match self.0 {
            ScheduleMoment::Fixed(ref moment) =>
                moment.create_timestamp(ut_midnight_reference, localtime),
            ScheduleMoment::Fuzzy(ref m1, ref m2) => {
                // pick a time between both given moment
                let mut rng = rand::thread_rng();
                let t1 = m1.create_timestamp(ut_midnight_reference, localtime);
                let t2 = m2.create_timestamp(ut_midnight_reference, localtime);
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
    // List of (abstract) moments in a day
    events: Vec<Rc<ScheduleEvent<'a>>>,

    // Time zone related information
    zoneinfo: ZoneInfo,

    // Next zone change
    localtime: LocalTimeState,

    // Tree of actual scheduled moments and reference to the abstract moment in a day
    schedule: BTreeMap<Timespec, Rc<ScheduleEvent<'a>>>
}

impl <'a>Schedule<'a> {
    /// Create a (empty) list of moments in a day
    pub fn new() -> Result<Schedule<'a>> {
        Ok(Schedule {
            events: vec![],
            zoneinfo: try!(ZoneInfo::get_local_zoneinfo()),
            localtime: LocalTimeState::Unknown,
            schedule: BTreeMap::new()
        })
    }

    /// Add a (abstract) moment and action in a day
    pub fn add_event(&mut self, moment: ScheduleMoment, action: &'a ScheduleAction) {
        self.events.push(Rc::new(ScheduleEvent(moment, action)));
    }

    /// Determine next zone info state
    fn new_change_state(&self, timestamp: Timespec) -> LocalTimeState {
        // yes, a unwrap, since a serious problem be present when no zoneinfo could be retrieved
        let actual = self.zoneinfo.get_actual_zoneinfo(timestamp).unwrap();
        match self.zoneinfo.get_next_transition_time(timestamp) {
            Some((next_change, next)) =>
                LocalTimeState::ChangePending(next_change, actual, next),
            None => LocalTimeState::NoChangePending(actual)
        }
    }

    /// Update the schedule for 24 hours
    pub fn update_schedule(&mut self, ut_midnight_reference: Timespec) {
        match self.localtime {
            LocalTimeState::Unknown =>
                self.localtime = self.new_change_state(ut_midnight_reference),
            LocalTimeState::ChangePending(time, _, _) => {
                if time >= ut_midnight_reference {
                    self.localtime = self.new_change_state(ut_midnight_reference);
                }
            },
            _ => {}
        }

        for event in &self.events {
            let timestamp = event.create_timestamp(ut_midnight_reference, &self.localtime);
            self.schedule.insert(timestamp, event.clone());
        }
    }

    /// Consume schedule until now and kick last or current event and returns next event time
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

    /// Peek when next event will happen
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
