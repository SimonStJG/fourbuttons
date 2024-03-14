use crate::{activity::Activity, schedule::Schedule};
use chrono::{Duration, NaiveDateTime};

const GRACE_PERIOD_HOURS: i64 = 1;

pub(crate) struct Scheduler {
    jobs: Vec<Job>,
}

pub(crate) struct ScheduledJobSpec {
    schedule: Schedule,
    activity: Activity,
}

struct Job {
    next_trigger: NaiveDateTime,
    schedule: Schedule,
    activity: Activity,
}

impl Scheduler {
    pub(crate) fn new(now: NaiveDateTime, job_specs: Vec<ScheduledJobSpec>) -> Self {
        Self {
            jobs: job_specs
                .iter()
                .map(|spec| Job {
                    // TODO I still don't really have a good feel for when to use this clone?
                    // Perhaps actually it's because the whole use of Vec is unnecessary for the schedule?
                    // Could I use a bitfield?
                    schedule: spec.schedule.clone(),
                    activity: spec.activity,
                    next_trigger: spec.schedule.calculate_next_trigger(now),
                })
                .collect(),
        }
    }

    pub(crate) fn tick(self: &mut Self, now: NaiveDateTime) -> Vec<Activity> {
        self.jobs.iter_mut().flat_map(|job| job.tick(now)).collect()
    }
}

impl ScheduledJobSpec {
    pub(crate) fn new(schedule: Schedule, activity: Activity) -> Self {
        Self { schedule, activity }
    }
}

impl Job {
    fn tick(self: &mut Self, now: NaiveDateTime) -> Option<Activity> {
        println!("{} {}", now, self.next_trigger);
        if now - self.next_trigger > Duration::hours(GRACE_PERIOD_HOURS) {
            // It's been so long since the last tick that we don't want to
            // trigger.  Just reset and wait for the next one.
            self.next_trigger = self.schedule.calculate_next_trigger(now);
            return None;
        } else if now >= self.next_trigger {
            self.next_trigger = self.schedule.calculate_next_trigger(now);
            return Some(self.activity);
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::{NaiveDateTime, NaiveTime};

    use crate::{
        activity::Activity,
        schedule::{every_day, DailySchedule, Schedule},
    };

    use super::{ScheduledJobSpec, Scheduler};

    #[test]
    fn regular_ticks() {
        let now = NaiveDateTime::from_str("2020-01-01T07:59:00").unwrap();
        let job_spec = ScheduledJobSpec::new(
            Schedule::Daily(DailySchedule::new(
                NaiveTime::from_str("08:00:00").unwrap(),
                every_day(),
            )),
            Activity::I,
        );
        let mut sched = Scheduler::new(now, vec![job_spec]);

        assert_eq!(sched.tick(now), vec![]);
        // Advance to scheduled time, see activity
        let now = NaiveDateTime::from_str("2020-01-01T08:00:00").unwrap();
        assert_eq!(sched.tick(now), vec![Activity::I]);

        // Run again at scheduled time, don't see activity
        let now = NaiveDateTime::from_str("2020-01-01T08:00:00").unwrap();
        assert_eq!(sched.tick(now), vec![]);

        // Advance past scheduled time
        let now = NaiveDateTime::from_str("2020-01-01T08:00:01").unwrap();
        assert_eq!(sched.tick(now), vec![]);
    }

    #[test]
    fn within_grace_period() {
        let now = NaiveDateTime::from_str("2020-01-01T07:59:00").unwrap();
        let job_spec = ScheduledJobSpec::new(
            Schedule::Daily(DailySchedule::new(
                NaiveTime::from_str("08:00:00").unwrap(),
                every_day(),
            )),
            Activity::I,
        );
        let mut sched = Scheduler::new(now, vec![job_spec]);

        // Just before end of grace period
        let now = NaiveDateTime::from_str("2020-01-01T09:00:00").unwrap();
        assert_eq!(sched.tick(now), vec![Activity::I]);
    }

    #[test]
    fn outside_of_grace_period() {
        let now = NaiveDateTime::from_str("2020-01-01T07:59:00").unwrap();
        let job_spec = ScheduledJobSpec::new(
            Schedule::Daily(DailySchedule::new(
                NaiveTime::from_str("08:00:00").unwrap(),
                every_day(),
            )),
            Activity::I,
        );
        let mut sched = Scheduler::new(now, vec![job_spec]);

        // Just outside of grace period
        let now = NaiveDateTime::from_str("2020-01-01T09:00:01").unwrap();
        assert_eq!(sched.tick(now), vec![]);
    }
}
