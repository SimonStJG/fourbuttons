#[allow(unused_imports)]
use log::{debug, error, info, warn};
use rusqlite::{Connection, Error, Result};

#[derive(PartialEq, Debug)]
pub(crate) struct Reading {
    id: Option<i32>,
    event_id: i32,
}

impl Reading {
    pub(crate) fn new(event_id: i32) -> Reading {
        Reading { id: None, event_id }
    }
}

pub(crate) struct Db {}

impl Db {
    pub(crate) fn new() -> Result<Db, Error> {
        info!("Initialising DB");
        let db = Db {};
        let conn = new_conn()?;
        conn.execute(
            "
                CREATE TABLE IF NOT EXISTS reading (
                id        INTEGER PRIMARY KEY,
                event_id  TEXT NOT NULL
            )",
            (),
        )?;

        Ok(db)
    }

    pub(crate) fn insert_reading(self: &Self, to_insert: &Reading) -> Result<()> {
        debug!("Inserting reading into DB: {:?}", to_insert);
        let conn = new_conn()?;
        conn.execute(
            "INSERT INTO reading (event_id) VALUES (:event_id)",
            &[(":event_id", &to_insert.event_id)],
        )
        .map(|_count| ())
    }
}

fn new_conn() -> Result<Connection, Error> {
    Connection::open("./db")
}
