use chrono::NaiveDateTime;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rusqlite::{Connection, Error, OptionalExtension, Result};

use crate::ApplicationState;

static TABLE_MIGRATIONS: &str = "migrations";
static SQLITE_DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S.%fZ";

struct Migration<'a> {
    id: &'a str,
    sql: &'a str,
}

static MIGRATIONS: [Migration; 1] = [Migration {
    id: "001",
    sql: "CREATE TABLE IF NOT EXISTS application_state (
                  id                    INTEGER PRIMARY KEY
                , take_pills_pending    TIMESTAMP
                , water_plants_pending  TIMETSTAMP
                , created_on            DEFAULT CURRENT_TIMESTAMP
            )",
}];

pub(crate) struct Db {}

impl Db {
    pub(crate) fn new() -> Result<Db, Error> {
        debug!("Initialising DB");
        let db = Db {};

        let migrations_to_run = calculate_migrations_to_run()?;
        run_migrations(migrations_to_run)?;

        Ok(db)
    }

    pub(crate) fn update_application_state(
        self: &Self,
        application_state: &ApplicationState,
    ) -> Result<()> {
        let conn = new_conn()?;
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

    pub(crate) fn load_application_state(self: &Self) -> Result<Option<ApplicationState>> {
        let conn = new_conn()?;
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
            Some((take_pills, water_plants)) => Ok(Some(ApplicationState {
                take_pills_pending: take_pills
                    .map(|dt: String| parse_naivedatetime_from_sqlite(&dt)),
                water_plants_pending: water_plants
                    .map(|dt: String| parse_naivedatetime_from_sqlite(&dt)),
            })),
            None => Ok(None),
        }
    }
}

fn calculate_migrations_to_run() -> Result<&'static [Migration<'static>]> {
    let conn = new_conn()?;

    let migrations_table_exists = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=:name",
            &[(":name", TABLE_MIGRATIONS)],
            |row| row.get::<usize, String>(0),
        )
        .optional()?
        .is_some();

    // TODO Tidy up all this crap and add some proper logs
    info!("Exists? {}", migrations_table_exists);

    let current_migration = if !migrations_table_exists {
        conn.execute(
            "
                CREATE TABLE migrations (
                  id           INTEGER PRIMARY KEY
                , migration_id TEXT
                , created_on   DEFAULT CURRENT_TIMESTAMP
            )
        ",
            (),
        )?;
        None
    } else {
        conn.query_row(
            "
                SELECT migration_id 
                FROM migrations 
                ORDER BY id DESC
                LIMIT 1
            ",
            (),
            |row| row.get::<usize, String>(0),
        )
        .optional()?
    };

    info!("current migration? {:?}", current_migration);

    return match current_migration {
        Some(current_migration_id) => {
            // TODO Unwrap
            let migration_idx = MIGRATIONS
                .iter()
                .position(|m| m.id == current_migration_id)
                .unwrap();
            Ok(&MIGRATIONS[migration_idx + 1..])
        }
        None => Ok(MIGRATIONS.as_slice()),
    };
}

fn run_migrations(migrations: &[Migration]) -> Result<()> {
    for migration in migrations {
        info!("Running migration {}", migration.id);
        let conn = new_conn()?;
        conn.execute(migration.sql, ())?;
        conn.execute(
            "
                INSERT INTO migrations (migration_id)
                VALUES (?1)
            ",
            &[&migration.id],
        )?;
    }
    return Ok(());
}

fn parse_naivedatetime_from_sqlite(encoded: &str) -> NaiveDateTime {
    // TODO Get rid of this unwrap
    return NaiveDateTime::parse_from_str(encoded, SQLITE_DATETIME_FMT).unwrap();
}

fn fmt_naivedatetime_for_sqlite(datetime: &NaiveDateTime) -> String {
    return datetime.format(SQLITE_DATETIME_FMT).to_string();
}

fn new_conn() -> Result<Connection, Error> {
    Connection::open("./db")
}
