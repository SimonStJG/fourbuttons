use std::ops::Add;

use chrono::{Datelike, Days, Duration, NaiveDateTime, NaiveTime, Weekday};

use crate::activity::Activity;

const GRACE_PERIOD_HOURS: i64 = 1;

pub(crate) struct Scheduler {
    jobs: Vec<Job>,
}

impl Scheduler {
    pub(crate) fn new(now: NaiveDateTime) -> Self {
        Self {
            jobs: vec![
                Job::new(
                    now,
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).unwrap(),
                    every_day(),
                    Activity::TakePills,
                ),
                Job::new(
                    now,
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).unwrap(),
                    vec![Weekday::Sat],
                    Activity::WaterPlants,
                ),
            ],
        }
    }

    pub(crate) fn tick(self: &mut Self, now: NaiveDateTime) -> Vec<Activity> {
        self.jobs.iter_mut().flat_map(|job| job.tick(now)).collect()
    }
}

fn every_day() -> Vec<Weekday> {
    vec![
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
}

struct Job {
    next_trigger: NaiveDateTime,
    schedule_time: NaiveTime,
    schedule_days: Vec<Weekday>,
    activity: Activity,
}

impl Job {
    pub(crate) fn new(
        now: NaiveDateTime,
        schedule_time: NaiveTime,
        schedule_days: Vec<Weekday>,
        activity: Activity,
    ) -> Self {
        Self {
            next_trigger: calculate_next_trigger(schedule_time, &schedule_days, now),
            schedule_time,
            schedule_days,
            activity,
        }
    }

    pub(crate) fn tick(self: &mut Self, now: NaiveDateTime) -> Option<Activity> {
        if now - self.next_trigger > Duration::hours(GRACE_PERIOD_HOURS) {
            // It's been so long since the last tick that we don't want to
            // trigger.  Just reset and wait for the next one.
            self.next_trigger =
                calculate_next_trigger(self.schedule_time, &self.schedule_days, now);
            return None;
        } else if now > self.next_trigger {
            self.next_trigger =
                calculate_next_trigger(self.schedule_time, &self.schedule_days, now);
            return Some(self.activity);
        }
        return None;
    }
}

fn calculate_next_trigger(
    schedule_time: NaiveTime,
    schedule_days: &Vec<Weekday>,
    now: NaiveDateTime,
) -> NaiveDateTime {
    let num_days_from_monday = now.weekday().num_days_from_monday();

    let next_weekday = if now.time() < schedule_time {
        schedule_days
            .iter()
            .find(|day| num_days_from_monday <= day.num_days_from_monday())
    } else {
        schedule_days
            .iter()
            .find(|day| num_days_from_monday < day.num_days_from_monday())
    }
    // Safe to unwrap because we really do know the schedule_days
    // is always non-empty
    .unwrap_or(schedule_days.get(0).unwrap());

    // TODO All of this unwrapping looks awful, but what's the correct way to do it?
    //  Is there some kind of magic function which will cast i32 to u32 only if > 0?
    let days_to_advance =
        days_from_monday(next_weekday) - TryInto::<i32>::try_into(num_days_from_monday).unwrap();
    let next_trigger_date = if days_to_advance >= 0 {
        now.date()
            .add(Days::new(days_to_advance.try_into().unwrap()))
    } else {
        now.date()
            .add(Days::new((days_to_advance + 7).try_into().unwrap()))
    };

    return NaiveDateTime::new(next_trigger_date, schedule_time);
}

fn days_from_monday(weekday: &Weekday) -> i32 {
    // Like num_days_from_monday except returns as i32 which I need for all
    // of my signed arithmetic!
    return TryInto::<i32>::try_into(weekday.num_days_from_monday()).unwrap();
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::{Datelike, NaiveDateTime, NaiveTime, Weekday};

    use crate::scheduler::{calculate_next_trigger, every_day};

    #[test]
    fn calculate_next_trigger_same_day() {
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("09:00:00").unwrap(),
                &every_day(),
                NaiveDateTime::from_str("2020-01-01T08:00:00").unwrap()
            ),
            NaiveDateTime::from_str("2020-01-01T09:00:00").unwrap()
        );
    }

    #[test]
    fn calculate_next_trigger_next_day() {
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("09:00:00").unwrap(),
                &every_day(),
                NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap()
            ),
            NaiveDateTime::from_str("2020-01-02T09:00:00").unwrap()
        );
    }

    #[test]
    fn calculate_next_trigger_next_day_week_boundary() {
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("09:00:00").unwrap(),
                &every_day(),
                NaiveDateTime::from_str("2020-01-05T10:00:00").unwrap()
            ),
            NaiveDateTime::from_str("2020-01-06T09:00:00").unwrap()
        );
    }

    #[test]
    fn calculate_next_trigger_same_week() {
        // Use weds as the test day
        let now = NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        // Next trigger is in the same week but earlier
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("08:00:00").unwrap(),
                &vec![Weekday::Fri],
                now
            ),
            NaiveDateTime::from_str("2020-01-03T08:00:00").unwrap()
        );

        // Next trigger is in the same week but later
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("10:00:00").unwrap(),
                &vec![Weekday::Fri],
                now
            ),
            NaiveDateTime::from_str("2020-01-03T10:00:00").unwrap()
        );
    }

    #[test]
    fn calculate_next_trigger_next_week() {
        let now = NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        // Next trigger is in the next week but earlier
        assert_eq!(
            NaiveDateTime::from_str("2020-01-07T10:00:00")
                .unwrap()
                .weekday(),
            Weekday::Tue
        );
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("08:00:00").unwrap(),
                &vec![Weekday::Tue],
                now
            ),
            NaiveDateTime::from_str("2020-01-07T08:00:00").unwrap()
        );
        // Next trigger is in the next week but later
        assert_eq!(
            calculate_next_trigger(
                NaiveTime::from_str("10:00:00").unwrap(),
                &vec![Weekday::Tue],
                now
            ),
            NaiveDateTime::from_str("2020-01-07T10:00:00").unwrap()
        );
    }
}
