#![deny(warnings)]

mod activity;
mod appdb;
mod db;
mod ledstrategy;
mod rpi;
mod schedule;
mod scheduler;

use appdb::AppDb;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use ledstrategy::LedState;
use log::{debug, error, info};
use rpi::{initialise_rpi, Button, Led, RpiInput, RpiOutput};
use scheduler::Scheduler;
use std::{
    error::Error,
    prelude::v1::Result,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use crate::{
    activity::Activity,
    schedule::{every_day, DailySchedule, Schedule, WeeklySchedule},
    scheduler::ScheduledJobSpec,
};

#[derive(Debug, PartialEq)]
struct ApplicationState {
    take_pills_pending: Option<NaiveDateTime>,
    water_plants_pending: Option<NaiveDateTime>,
    i_pending: Option<NaiveDateTime>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct LedStateChange {
    led: Led,
    state: LedState,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct InputError {}

type InputResult = Result<Button, InputError>;

fn rppal_thread_target(
    mut rpi: Box<dyn RpiInput + Send>,
    tx: &Sender<InputResult>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let button = rpi.wait_for_button_press()?;
        debug!("Sending: {:?}", button);
        let send_result = tx.send(Ok(button));
        debug!("Result: {:?}", send_result);
        if let Err(_) = send_result {
            // The only send error is receiver disconnected, so we can shut the thread down
            // cleanly
            return Ok(());
        }
    }
}

fn spawn_rppal_thread(
    rpi: Box<dyn RpiInput + Send>,
    tx: Sender<InputResult>,
) -> Result<thread::JoinHandle<()>, std::io::Error> {
    thread::Builder::new()
        .name("rppal".to_string())
        .spawn(move || {
            match rppal_thread_target(rpi, &tx) {
                Ok(_) => {
                    info!("rppal thread shutting down cleanly");
                }
                Err(err) => {
                    // We can't easily send any old dyn Error over the channel because we
                    // can't guaruntee it has the Send trait, so log instead.
                    error!("rppal thread died with err {}", err);
                    // If this fails it's because the receiver has already shut down.
                    let send_result = tx.send(Err(InputError {}));
                    debug!("rppal thread shutdown send result: {:?}", send_result);
                }
            }
        })
}

fn spawn_led_thread(
    mut rpi: Box<dyn RpiOutput + Send>,
    rx_timer: Receiver<Instant>,
    rx: Receiver<LedStateChange>,
) -> Result<thread::JoinHandle<()>, std::io::Error> {
    thread::Builder::new()
        .name("led".to_string())
        .spawn(move || {
            let mut strategies = ledstrategy::LedStrategies::all_off(&mut *rpi);

            loop {
                select! {
                    recv(rx_timer) -> timer_result => {
                        match timer_result {
                            Ok(instant) => {
                                strategies.tick(instant, &mut *rpi);
                            },
                            Err(_) => {
                                error!("rx_timer disconnect on LED thread");
                                break
                            }
                        }
                    },
                    recv(rx) -> rx_result => {
                        match rx_result {
                            Ok(state_change) => {
                                strategies.update(&mut *rpi, state_change.led, state_change.state);
                            }
                            Err(_) => {
                                info!("rx disconnect on LED thread");
                                break
                            }
                        }
                    },
                }
            }
        })
}

fn main_loop(
    db: AppDb,
    scheduler: &mut Scheduler,
    rx_timer: Receiver<Instant>,
    rx_input: Receiver<Result<Button, InputError>>,
    tx_led: Sender<LedStateChange>,
) {
    let mut application_state =
        db.load_application_state()
            .unwrap()
            .unwrap_or_else(|| ApplicationState {
                take_pills_pending: None,
                water_plants_pending: None,
                i_pending: None,
            });
    info!("Loaded state {:?}", application_state);
    // TODO De-duplicate with the logic in the main loop!!
    // TODO Try to write some tests for this main logic?
    if application_state.take_pills_pending.is_some() {
        tx_led
            .send(LedStateChange {
                led: Led::L1,
                state: LedState::On,
            })
            .unwrap();
    }
    if application_state.water_plants_pending.is_some() {
        tx_led
            .send(LedStateChange {
                led: Led::L4,
                state: LedState::On,
            })
            .unwrap();
    }
    if application_state.i_pending.is_some() {
        tx_led
            .send(LedStateChange {
                led: Led::L3,
                state: LedState::On,
            })
            .unwrap();
    }

    loop {
        select! {
            recv(rx_timer) -> instant_result => {
                match instant_result {
                    Ok(_) => {
                        main_loop_tick(scheduler, &mut application_state, &tx_led, &db);
                    }
                    Err(err) => {
                        info!("scheduler disconnected ({}), shutting down", err);
                        break
                    }
                }
            },
            recv(rx_input) -> input_result => {
                match input_result {
                    Ok(input) => {
                        match input {
                            Ok(button) => {
                                if !main_loop_on_btn_input(&button, &mut application_state, &tx_led, &db) {
                                    break;
                                }
                            },
                            Err(_) => panic!("Input error on rx_input"),
                        }
                    }
                    Err(err) => {
                        info!("rx_input disconnected ({}), shutting down", err);
                        break
                    }
                }
            }
        }
    }
}

fn main_loop_tick(
    scheduler: &mut Scheduler,
    application_state: &mut ApplicationState,
    tx_led: &Sender<LedStateChange>,
    db: &AppDb,
) {
    // TODO Do we really need to check the time all the time?
    let now = Utc::now().naive_local();
    for activity in scheduler.tick(now) {
        match activity {
            activity::Activity::TakePills => {
                application_state.take_pills_pending = Some(now);
                tx_led
                    .send(LedStateChange {
                        led: Led::L1,
                        state: LedState::On,
                    })
                    .unwrap();
            }
            activity::Activity::WaterPlants => {
                application_state.water_plants_pending = Some(now);
                tx_led
                    .send(LedStateChange {
                        led: Led::L4,
                        state: LedState::On,
                    })
                    .unwrap();
            }
            activity::Activity::I => {
                application_state.i_pending = Some(now);
                tx_led
                    .send(LedStateChange {
                        led: Led::L3,
                        state: LedState::On,
                    })
                    .unwrap();
            }
        }
    }

    db.update_application_state(application_state).unwrap();
}

fn main_loop_on_btn_input(
    button: &Button,
    application_state: &mut ApplicationState,
    tx_led: &Sender<LedStateChange>,
    db: &AppDb,
) -> bool {
    info!("Saw button press {:?}", button);
    // Whichever button is pressed, flash it
    // Sent any pending application state to not pending
    let led = match button {
        Button::B1 => Led::L1,
        Button::B2 => Led::L2,
        Button::B3 => Led::L3,
        Button::B4 => Led::L4,
        Button::STOP => {
            return false;
        }
    };

    // Important to do this first otherwise it feels laggy
    // (the db.insert_reading function called later is
    // blocking).
    tx_led
        .send(LedStateChange {
            led,
            state: LedState::BlinkTemporary,
        })
        .unwrap();

    match button {
        Button::B1 => {
            application_state.take_pills_pending = None;
        }
        Button::B2 => {}
        Button::B3 => {
            application_state.i_pending = None;
        }
        Button::B4 => {
            application_state.water_plants_pending = None;
        }
        Button::STOP => {}
    };

    db.update_application_state(application_state).unwrap();

    return true;
}

fn main() -> () {
    env_logger::init();
    info!("Initialising");
    let db = AppDb::new("./db".to_string());
    db.run_migrations().unwrap();

    let (rpi_input, rpi_output) = initialise_rpi().unwrap();

    let rx_input = {
        let (tx, rx) = unbounded::<InputResult>();
        spawn_rppal_thread(rpi_input, tx.clone()).unwrap();
        rx
    };
    let tx_led = {
        let (tx, rx) = unbounded::<LedStateChange>();
        spawn_led_thread(rpi_output, tick(Duration::from_millis(10)), rx).unwrap();
        tx
    };
    let now = Utc::now().naive_local();
    let mut scheduler = Scheduler::new(
        now,
        vec![
            ScheduledJobSpec::new(
                Schedule::Daily(DailySchedule::new(
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).expect("Invalid schedule"),
                    every_day(),
                )),
                Activity::TakePills,
            ),
            ScheduledJobSpec::new(
                Schedule::Daily(DailySchedule::new(
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).expect("Invalid schedule"),
                    vec![Weekday::Sat],
                )),
                Activity::WaterPlants,
            ),
            ScheduledJobSpec::new(
                Schedule::Weekly(WeeklySchedule::new(
                    NaiveDate::from_str("2024-03-14").unwrap(),
                    NaiveTime::from_hms_milli_opt(6, 0, 0, 0).expect("Invalid schedule"),
                    2,
                )),
                Activity::I,
            ),
        ],
    );

    info!("Entering main loop");
    main_loop(
        db,
        &mut scheduler,
        tick(Duration::from_millis(1000)),
        rx_input,
        tx_led,
    );
}
