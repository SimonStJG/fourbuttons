#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

mod activity;
mod actor;
mod appdb;
mod application_state;
mod db;
mod email;
mod ledstrategy;
mod rpi;
mod schedule;
mod scheduler;
mod supervisor;

use anyhow::{Context, Result};
use appdb::AppDb;
use chrono::{Duration, Local, NaiveDate, NaiveTime, Weekday};
use log::info;
use rpi::initialise_rpi;
use scheduler::Scheduler;
use std::{fs, str::FromStr, time::Instant};
use supervisor::supervisor::Supervisor;

use crate::{
    activity::Activity,
    actor::{
        control_actor::ControlActor,
        led_actor::{LedActor, LedActorMessage},
        rpi_input_actor::RpiInputActor,
        scheduler_actor::{SchedulerActor, SchedulerActorMessage},
        tick_actor::TickActor,
    },
    application_state::ApplicationState,
    email::Email,
    schedule::{every_day, DailySchedule, Schedule, WeeklySchedule},
    scheduler::ScheduledJobSpec,
};

fn main() {
    env_logger::init();
    info!("Initialising");
    let (db, email, application_state, rpi, scheduler) =
        initialise().expect("Initialisation error");
    info!("Running actors");
    run_actors(rpi, application_state, db, email, scheduler).expect("Abnormal shutdown");
}

fn initialise() -> Result<(AppDb, Email, ApplicationState, rpi::Rpi, Scheduler)> {
    let db = AppDb::new("./db".to_string());
    let mailgun_api_key =
        fs::read_to_string("./mailgun-apikey").context("Missing mailgun-apikey")?;
    let to_address = fs::read_to_string("./to-address").context("Missing to-address")?;
    let email = Email::new(
        mailgun_api_key.trim().to_owned(),
        to_address.trim().to_owned(),
    );

    db.run_migrations().context("Failed to run migrations")?;

    let application_state = db
        .load_application_state()
        .context("Failed to load application state")?
        .unwrap_or(ApplicationState::blank());
    info!("Loaded state {:?}", application_state);

    let rpi = initialise_rpi().context("Failed to initialise rpi")?;

    let now = Local::now().naive_local();
    let scheduler = Scheduler::new(
        now,
        &[
            ScheduledJobSpec::new(
                Schedule::Daily(DailySchedule::new(
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).context("Invalid schedule")?,
                    every_day(),
                )),
                Activity::TakePills,
                Duration::hours(1),
            ),
            ScheduledJobSpec::new(
                Schedule::Daily(DailySchedule::new(
                    NaiveTime::from_hms_milli_opt(11, 0, 0, 0).expect("Invalid schedule"),
                    every_day(),
                )),
                Activity::TakePillsReminder,
                Duration::hours(1),
            ),
            ScheduledJobSpec::new(
                Schedule::Daily(DailySchedule::new(
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).expect("Invalid schedule"),
                    vec![Weekday::Sat, Weekday::Wed],
                )),
                Activity::WaterPlants,
                Duration::hours(1),
            ),
            ScheduledJobSpec::new(
                Schedule::Weekly(WeeklySchedule::new(
                    NaiveDate::from_str("2024-03-13").expect("Invalid schedule start"),
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).expect("Invalid schedule"),
                    2,
                )),
                Activity::I,
                Duration::hours(12),
            ),
        ],
    );

    Ok((db, email, application_state, rpi, scheduler))
}

fn run_actors(
    rpi: rpi::Rpi,
    application_state: ApplicationState,
    db: AppDb,
    email: Email,
    scheduler: Scheduler,
) -> Result<()> {
    let mut supervisor = Supervisor::new();

    let tx_led = supervisor
        .start(LedActor::new(rpi.output), "LEDActor".to_owned())
        .context("Failed to start LED Actor")?;

    supervisor
        .start_message_source(
            TickActor::new(
                std::time::Duration::from_millis(10),
                tx_led.clone(),
                |instant: Instant| LedActorMessage::Tick(instant),
            ),
            "LED Tick Actor".to_owned(),
        )
        .context("Failed to start LED Tick Actor")?;

    let tx_control = supervisor
        .start(
            ControlActor::new(tx_led, application_state, db, email),
            "ControlActor".to_owned(),
        )
        .context("Failed to start Control Actor")?;

    supervisor
        .start_message_source(
            RpiInputActor::new(rpi.input, tx_control.clone()),
            "RPI Input Actor".to_owned(),
        )
        .context("Failed to start RPI Input Actor")?;

    let tx_scheduler = supervisor
        .start(
            SchedulerActor::new(scheduler, tx_control),
            "SchedulerActor".to_owned(),
        )
        .context("Failed to start Scheduler Actor")?;
    supervisor
        .start_message_source(
            TickActor::new(std::time::Duration::from_millis(1000), tx_scheduler, |_| {
                SchedulerActorMessage::Tick
            }),
            "Scheduler Tick Actor".to_owned(),
        )
        .context("Failed to start Scheduler Tick Actor")?;

    supervisor.supervise();

    Ok(())
}
