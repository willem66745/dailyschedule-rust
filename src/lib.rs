//! This crate provides functionality to execute a daily schedule for home
//! automation purposes. It provided a variety to schedule tasks in a day.
//!
//! The purpose of this crate is only calculation of execution times, but
//! doesn't perform the actual execution of the schedule loop.
//!
//! It doesn't rely on system time on purpose to allow easier testing and
//! qualification, without considering the real-time aspects. All
//! calculated timestamps are UTC based and any local-time conversion are
//! based on the zoneinfo crate.
extern crate rand;
extern crate time;
extern crate zoneinfo;

use time::{Timespec, Duration, at_utc};
use std::collections::BTreeMap;
use std::rc::Rc;
use rand::Rng;
use zoneinfo::{ZoneInfo, ZoneInfoElement};
use std::io::Result;

/// Represents a fixed moment in a day
pub enum Moment {
    /// Duration is offset in time based on local midnight
    LocalTime(Duration),
    /// Duration is offset in time based on UTC midnight
    UtcTime(Duration)
}

/// Local time definition
enum LocalTimeState {
    /// Zone-info state is not loaded yet
    Unknown,
    /// Current zone-info specifies no future daylight saving time change is expected
    NoChangePending(ZoneInfoElement),
    /// Daylight saving time change is pending
    ChangePending(Timespec, // transition time
                  ZoneInfoElement, // zone information before transition time
                  ZoneInfoElement) // zone information at and after transition time
}

impl Moment {
    /// Create a moment in a day
    pub fn new(h:u8, m:u8, s:u8) -> Moment {
        Moment::LocalTime(
            Duration::hours(h as i64) +
            Duration::minutes(m as i64) +
            Duration::seconds(s as i64))
    }

    /// Create a moment in a day based on Timespec
    pub fn new_from_timespec(ts: Timespec) -> Moment {
        let mut tm_utc = at_utc(ts);

        tm_utc.tm_hour = 0;
        tm_utc.tm_min = 0;
        tm_utc.tm_sec = 0;
        tm_utc.tm_nsec = 0;

        Moment::UtcTime(ts - tm_utc.to_timespec())
    }

    /// Convert schedule time to actual time stamp
    fn create_timestamp(&self, ut_midnight_reference: Timespec,
                        localtime: &LocalTimeState) -> Timespec {
        match self {
            // timestamp is simply a reference to UTC so just add the offset
            &Moment::UtcTime(offset) => ut_midnight_reference + offset,
            // timestamp is a reference to the moment in a day
            &Moment::LocalTime(offset) => { 
                let pre_localtime_cor = ut_midnight_reference + offset;

                let ut_offset = match *localtime {
                    LocalTimeState::NoChangePending(ref info) => info.ut_offset,
                    LocalTimeState::ChangePending(transition_time, ref before, ref after) => {
                        let reftime = Timespec::new(pre_localtime_cor.sec - before.ut_offset as i64,
                                                    pre_localtime_cor.nsec);
                        if reftime < transition_time {
                            before.ut_offset
                        } else {
                            after.ut_offset
                        }
                    }
                    _ => unreachable!()
                };

                Timespec::new(pre_localtime_cor.sec - ut_offset as i64, pre_localtime_cor.nsec)
            }
        }
    }
}

impl std::fmt::Debug for Moment {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let duration = match self {
            &Moment::UtcTime(d) => d,
            &Moment::LocalTime(d) => d,
        };
        try!(write!(fmt, "{:02}:{:02}:{:02}", duration.num_hours(), duration.num_minutes() % 60, duration.num_seconds() % 60));
        if let &Moment::UtcTime(_) = self {
            try!(write!(fmt, " (UTC)"));
        }
        Ok(())
    }
}

/// Weekday filter specifier
pub enum Filter {
    /// Always execute  event
    Always,
    /// Only execute Monday till Friday
    MonToFri,
    /// Only execute Saturday and Sunday
    Weekend // FIXME: more abstractions?
}

impl Filter {
    /// Indicate whether given time is valid to be scheduled based on weekday
    fn filter_days(&self, time: Timespec, zoneinfo: &ZoneInfoElement) -> bool {
        // make sure reference time is in the same weekday in UTC as it would be
        // in local time.
        let ref_time = Timespec::new(time.sec + zoneinfo.ut_offset as i64, time.nsec);
        let wday = at_utc(ref_time).tm_wday;
        let weekend = wday == 0 || wday == 6; // 0 = Sunday, 6 = Saturday

        match self {
            &Filter::Always => true,
            &Filter::MonToFri => !weekend,
            &Filter::Weekend => weekend
        }
    }

    /// Indicate whether given time is valid to be scheduled based on weekday
    fn day_scheduled(&self, time: Timespec, localtime: &LocalTimeState) -> bool {
        match self {
            &Filter::Always => true,
            &Filter::MonToFri|&Filter::Weekend => {
                let zoneinfo = match localtime {
                    &LocalTimeState::NoChangePending(ref zoneinfo) => zoneinfo,
                    &LocalTimeState::ChangePending(ref transition, ref z1, ref z2) => {
                        if time < *transition {
                            z1
                        } else {
                            z2
                        }
                    }
                    _ => unreachable!()
                };

                self.filter_days(time, zoneinfo)
            },
        }
    }
}

/// Represent a (abstract) moment in a day
pub enum DailyEvent {
    /// A fixed moment in a day
    Fixed(Filter, Moment),
    /// A random moment between two given fixed moments
    Fuzzy(Filter, Moment, Moment),
    /// A externally provided moment in time + variance
    ByClosure(Filter, Box<Fn(Timespec) -> Moment>, Duration)
}

impl std::fmt::Debug for DailyEvent {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &DailyEvent::Fixed(_, ref t) => write!(fmt, "Fixed {:?}", t),
            &DailyEvent::Fuzzy(_, ref b, ref a) => write!(fmt, "Fuzzy {:?} ~ {:?}", b, a),
            &DailyEvent::ByClosure(_, _, ref variance) =>
                write!(fmt, "ByClosure ~{:?}s", variance.num_seconds()),
        }
    }
}

/// Represents a moment and an specific action in a day
struct Event<C: Eq+PartialEq, H: Handler<C>> {
    /// A moment in a day
    moment: DailyEvent, 
    /// Reference to a action handler
    action: Rc<H>,
    /// Externally provided reference for the implementor
    context: C
}

impl<C: Eq+PartialEq, H: Handler<C>> Event<C, H> {
    /// Determine time-stamp for event
    fn create_timestamp(&self, ut_midnight_reference: Timespec,
                        localtime: &LocalTimeState) -> Option<Timespec> {
        let ts = match self.moment {
            DailyEvent::Fixed(_, ref moment) =>
                moment.create_timestamp(ut_midnight_reference, localtime),
            DailyEvent::Fuzzy(_, ref m1, ref m2) => {
                // pick a time between both given moment
                let mut rng = rand::thread_rng();
                let t1 = m1.create_timestamp(ut_midnight_reference, localtime);
                let t2 = m2.create_timestamp(ut_midnight_reference, localtime);
                let t_start = if t1 >= t2 {t2} else {t1};
                let t_end = if t1 >= t2 {t1} else {t2};
                let duration = t_end - t_start;
                if duration > Duration::seconds(0) {
                    t_start + Duration::seconds(rng.gen_range(0, duration.num_seconds()))
                } else {
                    t_start
                }
            }
            DailyEvent::ByClosure(_, ref func, ref variance) => {
                let moment = func(ut_midnight_reference);
                // generate a offset based on variance compared to the generated moment
                let mut rng = rand::thread_rng();
                let offset = if *variance > Duration::seconds(0) {
                    rng.gen_range(0, variance.num_seconds())
                } else {
                    0
                };
                let offset = Duration::seconds(variance.num_seconds() / 2 - offset);
                moment.create_timestamp(ut_midnight_reference, localtime) + offset
            }
        };
        let do_schedule = match self.moment {
            DailyEvent::Fixed(ref w, _) |
            DailyEvent::Fuzzy(ref w, _, _) |
            DailyEvent::ByClosure(ref w, _, _) => w.day_scheduled(ts, localtime)
        };

        if do_schedule {
            Some(ts)
        } else {
            None
        }
    }
}

/// Trait to be implemented by the event handler
pub trait Handler<C: Eq + PartialEq> {
    /// Hint for future events
    fn hint(&self, timestamp: &Timespec, context: &C);
    /// Perform a action (in a day)
    fn kick(&self, timestamp: &Timespec, context: &C);
}

/// Calculates and executes scheduled events every day
pub struct Schedule<C: Eq + PartialEq, H: Handler<C>> {
    // List of (abstract) moments in a day
    events: Vec<Rc<Event<C, H>>>,

    // Time zone related information
    zoneinfo: ZoneInfo,

    // Next zone change
    localtime: LocalTimeState,

    // Tree of actual scheduled moments and reference to the abstract moment in a day
    schedule: BTreeMap<Timespec, Vec<Rc<Event<C, H>>>>
}

impl<C: Eq + PartialEq, H: Handler<C>> Schedule<C, H> {
    /// Create a (empty) list of scheduled daily events
    pub fn new(zoneinfo: ZoneInfo) -> Schedule<C, H> {
        Schedule {
            events: vec![],
            zoneinfo: zoneinfo,
            localtime: LocalTimeState::Unknown,
            schedule: BTreeMap::new()
        }
    }

    /// Create a (empty) list of scheduled daily events based on the default zoneinfo (local time
    /// settings)
    pub fn new_local() -> Result<Schedule<C, H>> {
        Ok(Schedule::new(try!(ZoneInfo::get_local_zoneinfo())))
    }

    /// Add a (abstract) moment and action in a day
    pub fn add_event(&mut self,
                     moment: DailyEvent,
                     action: Rc<H>,
                     context: C) {
        self.events.push(Rc::new(Event {
            moment: moment,
            action: action,
            context: context
        }));
    }

    /// Determine next zone info state
    fn new_change_state(&self, timestamp: Timespec) -> LocalTimeState {
        // yes, a unwrap, since a serious problem be present when no zone-info could be retrieved
        let actual = self.zoneinfo.get_actual_zoneinfo(timestamp).unwrap();
        match self.zoneinfo.get_next_transition_time(timestamp) {
            Some((next_change, next)) =>
                LocalTimeState::ChangePending(next_change, actual, next),
            None => LocalTimeState::NoChangePending(actual)
        }
    }

    /// Update the schedule for 24 hours (only use with 24 hour incrementing timestamps,
    /// preferably every day)
    pub fn update_schedule(&mut self, ut_midnight_reference: Timespec) {
        match self.localtime {
            LocalTimeState::Unknown =>
                self.localtime = self.new_change_state(ut_midnight_reference),
            LocalTimeState::ChangePending(time, _, _) => {
                if time <= ut_midnight_reference {
                    self.localtime = self.new_change_state(ut_midnight_reference);
                }
            },
            _ => {}
        }

        for event in &self.events {
            let timestamp = event.create_timestamp(ut_midnight_reference, &self.localtime);
            if let Some(timestamp) = timestamp {
                event.action.hint(&timestamp, &event.context);

                let event_cloned = event.clone();

                if self.schedule.contains_key(&timestamp) {
                    self.schedule.get_mut(&timestamp).unwrap().push(event_cloned);
                } else {
                    self.schedule.insert(timestamp, vec![event_cloned]);
                }
            }
        }
    }

    /// Consume schedule until provided moment `now` and kick last or current event and returns next event time
    pub fn kick_event(&mut self, now: Timespec) -> Option<Timespec> {
        let past_events: Vec<Timespec> = self.schedule.keys().filter(|&k| *k <= now).cloned().collect();

        // kick the current event...
        for timestamp in past_events.iter() {
            if let Some(schedule_events) = self.schedule.get(&timestamp) {
                for schedule_event in schedule_events {
                    schedule_event.action.kick(&timestamp, &schedule_event.context);
                }
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
}
