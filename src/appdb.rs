use rusqlite::OptionalExtension;
use std::error::Error;

use crate::{
    db::{
        fmt_naivedatetime_for_sqlite, parse_naivedatetime_from_sqlite, Db, Migration,
        MigrationError,
    },
    ApplicationState,
};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration {
    id: "001",
    sql: "CREATE TABLE application_state (
                  id                    INTEGER PRIMARY KEY
                , take_pills_pending    TIMESTAMP
                , water_plants_pending  TIMETSTAMP
                , created_on            DEFAULT CURRENT_TIMESTAMP
            )",
}];

pub(crate) struct AppDb {
    db: Db,
}

impl AppDb {
    pub(crate) fn update_application_state(
        self: &Self,
        application_state: &ApplicationState,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.db.new_conn()?;
        let take_pills_pending: Option<String> = application_state
            .take_pills_pending
            .map(|dt| fmt_naivedatetime_for_sqlite(&dt));
        let water_plants_pending = application_state
            .water_plants_pending
            .map(|dt| fmt_naivedatetime_for_sqlite(&dt));
        conn.execute(
            "
                INSERT INTO application_state (
                    take_pills_pending
                  , water_plants_pending)
                VALUES (?1, ?2)
            ",
            &[&take_pills_pending, &water_plants_pending],
        )?;
        Ok(())
    }

    pub(crate) fn load_application_state(
        self: &Self,
        // TODO Stop boxing error
    ) -> Result<Option<ApplicationState>, Box<dyn Error>> {
        let conn = self.db.new_conn()?;
        let result = conn
            .query_row(
                "
                SELECT 
                      take_pills_pending
                    , water_plants_pending 
                FROM application_state
                ORDER BY id DESC
                LIMIT 1
            ",
                (),
                |row| {
                    Ok((
                        row.get::<usize, Option<String>>(0)?,
                        row.get::<usize, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?;

        match result {
            Some((take_pills, water_plants)) => {
                let take_pills_pending = take_pills
                    .map(|dt: String| parse_naivedatetime_from_sqlite(&dt))
                    .transpose()?;
                let water_plants_pending = water_plants
                    .map(|dt: String| parse_naivedatetime_from_sqlite(&dt))
                    .transpose()?;
                Ok(Some(ApplicationState {
                    take_pills_pending,
                    water_plants_pending,
                }))
            }
            None => Ok(None),
        }
    }

    pub(crate) fn new(path: String) -> Self {
        Self { db: Db::new(path) }
    }

    pub(crate) fn run_migrations(self: &Self) -> Result<(), MigrationError> {
        self.db.upgrade(MIGRATIONS)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDateTime;

    use crate::{db::Db, ApplicationState};

    use super::AppDb;

    impl AppDb {
        pub(crate) fn new_tmp() -> Self {
            Self { db: Db::new_tmp() }
        }
    }

    #[test]
    fn save_and_load_populated_app_state() {
        let appdb = AppDb::new_tmp();
        appdb.run_migrations().unwrap();

        let take_pills_pending = Some(NaiveDateTime::from_str("2020-01-01T08:00:00").unwrap());
        let water_plants_pending = Some(NaiveDateTime::from_str("2020-01-02T08:00:01").unwrap());
        let state = ApplicationState {
            take_pills_pending,
            water_plants_pending,
        };
        appdb.update_application_state(&state).unwrap();

        assert_eq!(appdb.load_application_state().unwrap().unwrap(), state);
    }


    #[test]
    fn save_and_load_empty_app_state() {
        let appdb = AppDb::new_tmp();
        appdb.run_migrations().unwrap();

        let state = ApplicationState {
            take_pills_pending: None,
            water_plants_pending: None,
        };
        appdb.update_application_state(&state).unwrap();

        assert_eq!(appdb.load_application_state().unwrap().unwrap(), state);
    }

    #[test]
    fn load_app_state_after_no_saves() {
        let appdb = AppDb::new_tmp();
        appdb.run_migrations().unwrap();

        assert_eq!(appdb.load_application_state().unwrap(), None);
    }
}
