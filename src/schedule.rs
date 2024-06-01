use chrono::{Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use std::ops::Add;

#[derive(Clone)]
pub(crate) struct DailySchedule {
    time: NaiveTime,
    days: Vec<Weekday>,
}

#[derive(Clone)]
pub(crate) struct WeeklySchedule {
    start_from: NaiveDate,
    repeat_every_n_weeks: u64,
    time: NaiveTime,
}

#[derive(Clone)]
pub(crate) enum Schedule {
    Daily(DailySchedule),
    Weekly(WeeklySchedule),
}

impl DailySchedule {
    pub(crate) fn new(schedule_time: NaiveTime, schedule_days: Vec<Weekday>) -> Self {
        Self {
            time: schedule_time,
            days: schedule_days,
        }
    }

    fn calculate_next_trigger(&self, now: NaiveDateTime) -> NaiveDateTime {
        let num_days_from_monday = now.weekday().num_days_from_monday();

        let next_weekday = if now.time() < self.time {
            self.days
                .iter()
                .find(|day| num_days_from_monday <= day.num_days_from_monday())
        } else {
            self.days
                .iter()
                .find(|day| num_days_from_monday < day.num_days_from_monday())
        }
        // Safe to unwrap because we really do know the schedule_days
        // is always non-empty
        .unwrap_or(self.days.first().unwrap());

        // TODO All of this unwrapping looks awful, but what's the correct way to do it?
        //  Is there some kind of magic function which will cast i32 to u32 only if > 0?
        let days_to_advance = days_from_monday(*next_weekday)
            - TryInto::<i32>::try_into(num_days_from_monday).unwrap();
        let next_trigger_date = if days_to_advance >= 0 {
            now.date()
                .add(Days::new(days_to_advance.try_into().unwrap()))
        } else {
            now.date()
                .add(Days::new((days_to_advance + 7).try_into().unwrap()))
        };

        NaiveDateTime::new(next_trigger_date, self.time)
    }
}

impl WeeklySchedule {
    pub(crate) fn new(
        schedule_start_from: NaiveDate,
        schedule_time: NaiveTime,
        schedule_repeat_every_n_weeks: u64,
    ) -> Self {
        Self {
            start_from: schedule_start_from,
            repeat_every_n_weeks: schedule_repeat_every_n_weeks,
            time: schedule_time,
        }
    }

    fn calculate_next_trigger(&self, now: NaiveDateTime) -> NaiveDateTime {
        let days_since_start_u: i64 = now
            .date()
            .signed_duration_since(self.start_from)
            .num_days();

        let days_since_start =
            u64::try_from(days_since_start_u).expect("Schedule start time in the future");

        let schedule_period_in_days = 7 * self.repeat_every_n_weeks;
        let remainder: u64 = days_since_start % schedule_period_in_days;

        let days_to_advance = if remainder == 0 && now.time() <= self.time {
            0
        } else {
            schedule_period_in_days - remainder
        };

        let trigger_date = now
            .date()
            .checked_add_days(Days::new(days_to_advance))
            .unwrap();
        NaiveDateTime::new(trigger_date, self.time)
    }
}

impl Schedule {
    pub(crate) fn calculate_next_trigger(&self, now: NaiveDateTime) -> NaiveDateTime {
        match self {
            Schedule::Daily(schedule) => schedule.calculate_next_trigger(now),
            Schedule::Weekly(schedule) => schedule.calculate_next_trigger(now),
        }
    }
}

pub(crate) fn every_day() -> Vec<Weekday> {
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

fn days_from_monday(weekday: Weekday) -> i32 {
    // Like num_days_from_monday except returns as i32 which I need for all
    // of my signed arithmetic!
    TryInto::<i32>::try_into(weekday.num_days_from_monday()).unwrap()
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

    use crate::schedule::{every_day, DailySchedule, Schedule, WeeklySchedule};

    #[test]
    fn daily_same_day() {
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("09:00:00").unwrap(),
            every_day(),
        ));
        assert_eq!(
            schedule
                .calculate_next_trigger(NaiveDateTime::from_str("2020-01-01T08:00:00").unwrap()),
            NaiveDateTime::from_str("2020-01-01T09:00:00").unwrap()
        );
    }

    #[test]
    fn daily_next_day() {
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("09:00:00").unwrap(),
            every_day(),
        ));
        assert_eq!(
            schedule
                .calculate_next_trigger(NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap()),
            NaiveDateTime::from_str("2020-01-02T09:00:00").unwrap()
        );
    }

    #[test]
    fn daily_next_day_week_boundary() {
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("09:00:00").unwrap(),
            every_day(),
        ));
        assert_eq!(
            schedule
                .calculate_next_trigger(NaiveDateTime::from_str("2020-01-05T10:00:00").unwrap()),
            NaiveDateTime::from_str("2020-01-06T09:00:00").unwrap()
        );
    }

    #[test]
    fn daily_same_week() {
        // Use weds as the test day
        let now = NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("08:00:00").unwrap(),
            vec![Weekday::Fri],
        ));
        // Next trigger is in the same week but earlier
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-03T08:00:00").unwrap()
        );

        // Next trigger is in the same week but later
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("10:00:00").unwrap(),
            vec![Weekday::Fri],
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-03T10:00:00").unwrap()
        );
    }

    #[test]
    fn daily_next_week() {
        let now = NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        // Next trigger is in the next week but earlier
        assert_eq!(
            NaiveDateTime::from_str("2020-01-07T10:00:00")
                .unwrap()
                .weekday(),
            Weekday::Tue
        );
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("08:00:00").unwrap(),
            vec![Weekday::Tue],
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-07T08:00:00").unwrap()
        );
        // Next trigger is in the next week but later
        let schedule = Schedule::Daily(DailySchedule::new(
            NaiveTime::from_str("10:00:00").unwrap(),
            vec![Weekday::Tue],
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-07T10:00:00").unwrap()
        );
    }

    // This is annoying to think about so here's a calendar for Jan 2020
    //
    //      Mon Tue Wed Thu Fri Sat Sun
    // Wk1          01  02  03  04  05
    // Wk2  06  07  08  09  10  11  12
    // Wk3  13  14  15  16  17  18  19
    // ...
    #[test]
    fn weekly_just_after_schedule_time() {
        let now = NaiveDateTime::from_str("2020-01-01T10:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        // Next trigger is 1 weeks away
        let schedule = Schedule::Weekly(WeeklySchedule::new(
            NaiveDate::from_str("2020-01-01").unwrap(),
            NaiveTime::from_str("08:00:00").unwrap(),
            2,
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-15T08:00:00").unwrap()
        );
    }

    #[test]
    fn weekly_just_before_schedule_time() {
        let now = NaiveDateTime::from_str("2020-01-15T06:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Wed);

        // Next trigger is 1 weeks away
        let schedule = Schedule::Weekly(WeeklySchedule::new(
            NaiveDate::from_str("2020-01-01").unwrap(),
            NaiveTime::from_str("08:00:00").unwrap(),
            2,
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-15T08:00:00").unwrap()
        );
    }

    #[test]
    fn weekly_two_days_before() {
        let now = NaiveDateTime::from_str("2020-01-13T06:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Mon);

        // Next trigger is 1 weeks away
        let schedule = Schedule::Weekly(WeeklySchedule::new(
            NaiveDate::from_str("2020-01-01").unwrap(),
            NaiveTime::from_str("08:00:00").unwrap(),
            2,
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-15T08:00:00").unwrap()
        );
    }

    #[test]
    fn weekly_two_days_after() {
        let now = NaiveDateTime::from_str("2020-01-03T06:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Fri);

        // Next trigger is 1 weeks away
        let schedule = Schedule::Weekly(WeeklySchedule::new(
            NaiveDate::from_str("2020-01-01").unwrap(),
            NaiveTime::from_str("08:00:00").unwrap(),
            2,
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-01-15T08:00:00").unwrap()
        );
    }

    #[test]
    fn weekly_large_gap() {
        let now = NaiveDateTime::from_str("2020-02-03T06:00:00").unwrap();
        assert_eq!(now.weekday(), Weekday::Mon);

        // Next trigger is 1 weeks away
        let schedule = Schedule::Weekly(WeeklySchedule::new(
            NaiveDate::from_str("2020-01-01").unwrap(),
            NaiveTime::from_str("08:00:00").unwrap(),
            2,
        ));
        assert_eq!(
            schedule.calculate_next_trigger(now),
            NaiveDateTime::from_str("2020-02-12T08:00:00").unwrap()
        );
    }
}
