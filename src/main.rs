#![deny(warnings)]

mod db;
mod ledstrategy;
mod rpi;
mod scheduler;

use crossbeam_channel::{select, tick, unbounded, Receiver, Sender};
use db::{Db, Reading};
use ledstrategy::LedState;
use log::{debug, error, info};
use rpi::{initialise_rpi, Button, Led, RpiInput, RpiOutput};
use scheduler::Scheduler;
use std::{
    error::Error,
    prelude::v1::Result,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, Copy, Clone)]
enum ButtonState {
    Pending,
    NotPending,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct ButtonStates {
    b1: ButtonState,
    b2: ButtonState,
    b3: ButtonState,
    b4: ButtonState,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct LedStateChange {
    led: Led,
    state: LedState,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct InputError {}

type InputResult = Result<Button, InputError>;

fn button_to_event_id(button: &Button) -> i32 {
    match *button {
        Button::B1 => 1,
        Button::B2 => 2,
        Button::B3 => 3,
        Button::B4 => 4,
        Button::STOP => 100,
    }
}

fn rppal_thread_target(
    mut rpi: Box<dyn RpiInput + Send>,
    tx: &Sender<InputResult>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let (button, _level) = rpi.poll_interrupts()?;
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
                                error!("rx_timer disconnect");
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
                                error!("rx disconnect");
                                break
                            }
                        }
                    },
                }
            }
            info!("LED Thread shutdown");
        })
}

fn main_loop(
    db: Db,
    mut scheduler: Scheduler,
    rx_timer: Receiver<Instant>,
    rx_input: Receiver<Result<Button, InputError>>,
    tx_led: Sender<LedStateChange>,
) {
    let mut button_states = ButtonStates {
        b1: ButtonState::NotPending,
        b2: ButtonState::NotPending,
        b3: ButtonState::NotPending,
        b4: ButtonState::NotPending,
    };

    loop {
        select! {
            recv(rx_timer) -> instant_result => {
                match instant_result {
                    Ok(_) => {
                        for activity in scheduler.tick() {
                            match activity {
                                scheduler::ScheduledActivity::TakePills => {
                                    button_states.b1 = ButtonState::Pending;
                                    tx_led.send(LedStateChange{
                                        led: Led::L1,
                                        state: LedState::On,
                                    }).unwrap();
                                },
                                scheduler::ScheduledActivity::WaterPlants => {
                                    button_states.b4 = ButtonState::Pending;
                                    tx_led.send(LedStateChange{
                                        led: Led::L4,
                                        state: LedState::On,
                                    }).unwrap();
                                },
                                scheduler::ScheduledActivity::TestActivity => {
                                    button_states.b3 = ButtonState::Pending;
                                    tx_led.send(LedStateChange{
                                        led: Led::L3,
                                        state: LedState::On,
                                    }).unwrap();
                                }
                            }
                        }
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
                                info!("Saw button press {:?}", button);
                                let event_id = button_to_event_id(&button);
                                db.insert_reading(&Reading::new(event_id)).unwrap();
                                // Whichever button is pressed, flash it
                                // If it was pending, set it to not pending
                                let (button_state, led) = match button {
                                    Button::B1 => (&mut button_states.b1, Led::L1),
                                    Button::B2 => (&mut button_states.b2, Led::L2),
                                    Button::B3 => (&mut button_states.b3, Led::L3),
                                    Button::B4 => (&mut button_states.b4, Led::L4),
                                    Button::STOP => {
                                        break
                                    },
                                };
                                if *button_state == ButtonState::Pending {
                                    *button_state = ButtonState::NotPending;
                                }
                                tx_led.send(LedStateChange{
                                    led: led,
                                    state: LedState::BlinkTemporary,
                                }).unwrap();
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

fn main() -> () {
    env_logger::init();
    let db = Db::new().unwrap();
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
    let scheduler = Scheduler::new();

    main_loop(
        db,
        scheduler,
        tick(Duration::from_millis(1000)),
        rx_input,
        tx_led,
    );
}
