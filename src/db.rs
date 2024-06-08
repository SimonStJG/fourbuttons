use std::{error::Error, fmt};

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use log::info;
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, PartialEq, PartialOrd)]
struct UnknownMigrationError(String);

impl Error for UnknownMigrationError {}

impl fmt::Display for UnknownMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "migration ID {} in database doesn't appear in migration history",
            self.0
        )
    }
}

static SQLITE_DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S.%fZ";

// A trait to make it easier to inject temporary database files when running
// tests.
trait DbFilePath {
    fn path(&self) -> String;
}

impl DbFilePath for String {
    fn path(&self) -> String {
        self.clone()
    }
}

pub(crate) struct Migration<'a> {
    pub(crate) id: &'a str,
    pub(crate) sql: &'a str,
}

pub(crate) struct Db {
    file_path: Box<dyn DbFilePath + Send + Sync + 'static>,
}

impl Db {
    pub(crate) fn new(file_path: String) -> Self {
        Self {
            file_path: Box::new(file_path),
        }
    }

    pub(crate) fn upgrade(&self, migrations: &[Migration]) -> Result<()> {
        let migrations_to_run = self.calculate_migrations_to_run(migrations)?;
        self.run_migrations(migrations_to_run)?;

        Ok(())
    }

    fn calculate_migrations_to_run<'a, 'b>(
        &self,
        migrations: &'a [Migration<'b>],
    ) -> Result<&'a [Migration<'b>]> {
        let conn = self.new_conn()?;

        let migrations_table_exists = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name=:name",
                &[(":name", "migrations")],
                |row| row.get::<usize, String>(0),
            )
            .optional()?
            .is_some();

        let current_migration = if migrations_table_exists {
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
        } else {
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
        };

        if let Some(current_migration_id) = current_migration {
            info!("Current DB migration: {}", current_migration_id);
            migrations
                .iter()
                .position(|m| m.id == current_migration_id)
                .map(|idx| &migrations[idx + 1..])
                .ok_or(UnknownMigrationError(current_migration_id).into())
        } else {
            info!("Current DB migration: None");
            Ok(migrations)
        }
    }

    pub(crate) fn new_conn(&self) -> Result<Connection> {
        Connection::open(self.file_path.path()).context("Failed to open new sqlite connection")
    }

    fn run_migrations(&self, migrations: &[Migration]) -> Result<()> {
        for migration in migrations {
            info!("Running migration {}", migration.id);
            let conn = self.new_conn()?;
            conn.execute(migration.sql, ())?;
            conn.execute(
                "
                    INSERT INTO migrations (migration_id)
                    VALUES (?1)
                ",
                [&migration.id],
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

    use std::fmt::Write;
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
        fn path(&self) -> String {
            self.path.clone()
        }
    }

    // A lazy random file name generator (seems all the useful random number
    // generators are in crates?)
    fn tmp_file_name() -> String {
        let mut rng = File::open("/dev/urandom").unwrap();
        let mut buffer = [0u8; 1];
        rng.read_exact(&mut buffer).unwrap();
        let mut s: String = String::new();
        for b in &buffer {
            write!(s, "{b:02x}").unwrap();
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

    use super::{Db, Migration, UnknownMigrationError};

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
        assert!(table_exists(&conn, "migrations"));
        assert!(table_exists(&conn, "t1"));
    }

    #[test]
    fn upgrade_from_zero_migrations() {
        let db = Db::new_tmp();
        // Create just the migrations table
        db.upgrade(&[]).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert!(!table_exists(&conn, "t1"));
        }

        // Now do a normal upgrade
        db.upgrade(MIGRATIONS).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert!(table_exists(&conn, "t1"));
        }
    }

    #[test]
    fn upgrade_from_one_migration() {
        let db = Db::new_tmp();

        // Apply the 001 migration
        db.upgrade(&MIGRATIONS[0..1]).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert!(table_exists(&conn, "t1"));
            assert!(!table_exists(&conn, "t2"));
        }

        // Apply the 002 migration
        db.upgrade(MIGRATIONS).unwrap();
        {
            let conn = db.new_conn().unwrap();
            assert!(table_exists(&conn, "t1"));
            assert!(table_exists(&conn, "t2"));
        }
    }

    #[test]
    fn fail_on_unknown_migration() {
        let db = Db::new_tmp();

        // Apply the 002 migration
        db.upgrade(MIGRATIONS).unwrap();

        {
            let conn = db.new_conn().unwrap();
            assert!(table_exists(&conn, "t2"));
        }

        // Apply migrations without 002 in the history
        let err: UnknownMigrationError = db
            .upgrade(&MIGRATIONS[0..1])
            .unwrap_err()
            .downcast()
            .unwrap();

        assert_eq!(err, UnknownMigrationError("002".to_owned()));
    }

    #[test]
    fn fail_on_invalid_migration() {
        let db = Db::new_tmp();

        let err: rusqlite::Error = db
            .upgrade(&[Migration {
                id: "001",
                sql: "oh no",
            }])
            .unwrap_err()
            .downcast()
            .unwrap();

        assert!(matches!(err, rusqlite::Error::SqlInputError { .. }));
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
