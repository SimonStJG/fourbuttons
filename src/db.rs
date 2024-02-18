use std::{error::Error, fmt};

use chrono::NaiveDateTime;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rusqlite::{Connection, OptionalExtension};

// TODO have a generic error type for my whole app??
#[derive(Debug, PartialEq)]
pub(crate) enum MigrationError {
    UnknownMigrationError(String),
    RusqliteError(rusqlite::Error),
}

impl From<rusqlite::Error> for MigrationError {
    #[cold]
    fn from(err: rusqlite::Error) -> MigrationError {
        MigrationError::RusqliteError(err)
    }
}

impl Error for MigrationError {}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::UnknownMigrationError(migration_id) => {
                write!(
                    f,
                    "migration ID {} in database doesn't appear in migration history",
                    migration_id
                )
            }
            MigrationError::RusqliteError(err) => err.fmt(f),
        }
    }
}

static SQLITE_DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S.%fZ";

// A trait to make it easier to inject temporary database files when running
// tests.
trait DbFilePath {
    fn path(self: &Self) -> String;
}

impl DbFilePath for String {
    fn path(self: &Self) -> String {
        self.to_owned()
    }
}

pub(crate) struct Migration<'a> {
    pub(crate) id: &'a str,
    pub(crate) sql: &'a str,
}

pub(crate) struct Db {
    file_path: Box<dyn DbFilePath>,
}

impl Db {
    pub(crate) fn new(file_path: String) -> Self {
        Self {
            file_path: Box::new(file_path),
        }
    }

    pub(crate) fn upgrade(self: &Self, migrations: &[Migration]) -> Result<(), MigrationError> {
        let migrations_to_run = self.calculate_migrations_to_run(migrations)?;
        self.run_migrations(migrations_to_run)?;

        Ok(())
    }

    fn calculate_migrations_to_run<'a, 'b>(
        self: &Self,
        migrations: &'a [Migration<'b>],
    ) -> Result<&'a [Migration<'b>], MigrationError> {
        let conn = self.new_conn()?;

        let migrations_table_exists = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name=:name",
                &[(":name", "migrations")],
                |row| row.get::<usize, String>(0),
            )
            .optional()?
            .is_some();

        let current_migration = if !migrations_table_exists {
            conn.execute(
                "
                    CREATE TABLE migrations (
                        id             INTEGER PRIMARY KEY
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

        match current_migration {
            Some(current_migration_id) => {
                info!("Current DB migration: {}", current_migration_id);
                migrations
                    .iter()
                    .position(|m| m.id == current_migration_id)
                    .map(|idx| &migrations[idx + 1..])
                    .ok_or(MigrationError::UnknownMigrationError(current_migration_id))
            }
            None => {
                info!("Current DB migration: None");
                Ok(migrations)
            }
        }
    }

    pub(crate) fn new_conn(self: &Self) -> Result<Connection, rusqlite::Error> {
        Ok(Connection::open(self.file_path.path())?)
    }

    fn run_migrations(self: &Self, migrations: &[Migration]) -> Result<(), MigrationError> {
        for migration in migrations {
            info!("Running migration {}", migration.id);
            let conn = self.new_conn()?;
            conn.execute(migration.sql, ())?;
            conn.execute(
                "
                    INSERT INTO migrations (migration_id)
                    VALUES (?1)
                ",
                &[&migration.id],
            )?;
        }

        Ok(())
    }
}

pub(crate) fn parse_naivedatetime_from_sqlite(
    encoded: &str,
) -> Result<NaiveDateTime, chrono::ParseError> {
    NaiveDateTime::parse_from_str(encoded, SQLITE_DATETIME_FMT)
}

pub(crate) fn fmt_naivedatetime_for_sqlite(datetime: &NaiveDateTime) -> String {
    datetime.format(SQLITE_DATETIME_FMT).to_string()
}

#[cfg(test)]
pub(crate) mod testhelper {

    use std::{env, fs::File, io::Read};

    use super::{Db, DbFilePath};

    // A lazy persons tmp file which drops as soon as it goes
    // out of scope
    pub(crate) struct TmpFile {
        path: String,
    }

    impl TmpFile {
        pub(crate) fn new() -> Self {
            Self {
                path: tmp_file_name(),
            }
        }
    }

    impl Drop for TmpFile {
        fn drop(&mut self) {
            std::fs::remove_file(&self.path).unwrap();
        }
    }

    impl DbFilePath for TmpFile {
        fn path(self: &Self) -> String {
            self.path.to_owned()
        }
    }

    // A lazy random file name generator (seems all the useful random number
    // generators are in crates?)
    fn tmp_file_name() -> String {
        let mut rng = File::open("/dev/urandom").unwrap();
        let mut buffer = [0u8; 1];
        rng.read_exact(&mut buffer).unwrap();
        let mut s: String = String::new();
        use std::fmt::Write;
        for b in buffer.iter() {
            write!(s, "{:02x}", b).unwrap();
        }

        env::temp_dir().join(s).to_str().unwrap().to_string()
    }

    impl Db {
        pub(crate) fn new_tmp() -> Self {
            Self {
                file_path: Box::new(TmpFile::new()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, OptionalExtension};

    use crate::db::MigrationError;

    use super::{Db, Migration};

    static MIGRATIONS: &[Migration] = &[
        Migration {
            id: "001",
            sql: "CREATE TABLE t1 (
                id    INTEGER PRIMARY KEY
              , v     INTEGER NOT NULL                
            )",
        },
        Migration {
            id: "002",
            sql: "CREATE TABLE t2 (
                id    INTEGER PRIMARY KEY
              , v1    INTEGER NOT NULL                
            )",
        },
    ];

    #[test]
    fn upgrade_from_empty_db() {
        let db = Db::new_tmp();

        db.upgrade(MIGRATIONS).unwrap();

        let conn = db.new_conn().unwrap();
        assert_eq!(table_exists(&conn, "migrations"), true);
        assert_eq!(table_exists(&conn, "t1"), true);
    }

    #[test]
    fn upgrade_from_zero_migrations() {
        let db = Db::new_tmp();
        // Create just the migrations table
        db.upgrade(&[]).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert_eq!(table_exists(&conn, "t1"), false);
        }

        // Now do a normal upgrade
        db.upgrade(MIGRATIONS).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert_eq!(table_exists(&conn, "t1"), true);
        }
    }

    #[test]
    fn upgrade_from_one_migration() {
        let db = Db::new_tmp();

        // Apply the 001 migration
        db.upgrade(&MIGRATIONS[0..1]).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert_eq!(table_exists(&conn, "t1"), true);
            assert_eq!(table_exists(&conn, "t2"), false);
        }

        // Apply the 002 migration
        db.upgrade(&MIGRATIONS).unwrap();
        {
            let conn = db.new_conn().unwrap();
            assert_eq!(table_exists(&conn, "t1"), true);
            assert_eq!(table_exists(&conn, "t2"), true);
        }
    }

    #[test]
    fn fail_on_unknown_migration() {
        let db = Db::new_tmp();

        // Apply the 002 migration
        db.upgrade(MIGRATIONS).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert_eq!(table_exists(&conn, "t2"), true);
        }

        // Apply migrations without 002 in the history
        assert_eq!(
            db.upgrade(&MIGRATIONS[0..1]).unwrap_err(),
            MigrationError::UnknownMigrationError("002".to_string())
        );
    }

    #[test]
    fn fail_on_invalid_migration() {
        let db = Db::new_tmp();

        let err = db
            .upgrade(&[Migration {
                id: "001",
                sql: "oh no",
            }])
            .unwrap_err();

        match err {
            MigrationError::UnknownMigrationError(_) => panic!(),
            MigrationError::RusqliteError(_) => {}
        }
    }

    fn table_exists(conn: &Connection, table_name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=:name",
            &[(":name", table_name)],
            |row| row.get::<usize, i32>(0),
        )
        .optional()
        .unwrap()
        .is_some()
    }
}
