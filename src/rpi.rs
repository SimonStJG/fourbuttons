use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use std::{
    error::Error,
    io::{self, Read, Stdin},
};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

const USE_REAL_RPI: bool = false;
const PIN_BUTTON_1: u8 = 2;
const PIN_BUTTON_2: u8 = 3;
const PIN_BUTTON_3: u8 = 20;
const PIN_BUTTON_4: u8 = 21;
const PIN_LED_1: u8 = 23;
const PIN_LED_2: u8 = 24;
const PIN_LED_3: u8 = 27;
const PIN_LED_4: u8 = 22;

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Button {
    B1,
    B2,
    B3,
    B4,
    // Special button to top the app
    STOP,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Led {
    L1,
    L2,
    L3,
    L4,
}

pub(crate) trait RpiInput {
    fn poll_interrupts(self: &mut Self)
        -> Result<(Button, rppal::gpio::Level), rppal::gpio::Error>;
}

pub(crate) trait RpiOutput {
    fn switch_led(self: &mut Self, led: Led, is_on: bool);
}

// We need +Send because this is going to be shared between threads later when used for I/O
pub(crate) fn initialise_rpi(
) -> Result<(Box<dyn RpiInput + Send>, Box<dyn RpiOutput + Send>), Box<dyn Error>> {
    if USE_REAL_RPI {
        let gpio: rppal::gpio::Gpio = rppal::gpio::Gpio::new()?;

        let mut btnpin1 = gpio.get(PIN_BUTTON_1)?.into_input();
        let mut btnpin2 = gpio.get(PIN_BUTTON_2)?.into_input();
        let mut btnpin3 = gpio.get(PIN_BUTTON_3)?.into_input();
        let mut btnpin4 = gpio.get(PIN_BUTTON_4)?.into_input();

        let ledpin1 = gpio.get(PIN_LED_1)?.into_output_low();
        let ledpin2 = gpio.get(PIN_LED_2)?.into_output_low();
        let ledpin3 = gpio.get(PIN_LED_3)?.into_output_low();
        let ledpin4 = gpio.get(PIN_LED_4)?.into_output_low();

        btnpin1.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;
        btnpin2.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;
        btnpin3.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;
        btnpin4.set_interrupt(rppal::gpio::Trigger::RisingEdge)?;

        return Ok((
            Box::new(RealRpiInput {
                gpio,
                btnpin1,
                btnpin2,
                btnpin3,
                btnpin4,
            }),
            Box::new(RealRpiOutput {
                ledpin1,
                ledpin2,
                ledpin3,
                ledpin4,
            }),
        ));
    } else {
        Ok((
            Box::new(FakeRpiInput { stdin: io::stdin() }),
            Box::new(FakeRpiOutput {}),
        ))
    }
}

struct RealRpiInput {
    gpio: Gpio,
    btnpin1: InputPin,
    btnpin2: InputPin,
    btnpin3: InputPin,
    btnpin4: InputPin,
}

impl RpiInput for RealRpiInput {
    fn poll_interrupts(
        self: &mut Self,
    ) -> Result<(Button, rppal::gpio::Level), rppal::gpio::Error> {
        match self.gpio.poll_interrupts(
            &[&self.btnpin1, &self.btnpin2, &self.btnpin3, &self.btnpin4],
            // Setting `reset` to `false` returns any cached interrupt trigger events if available.
            false,
            None,
        ) {
            Ok(Some((pin, level))) => match pin.pin() {
                1 => Ok((Button::B1, level)),
                2 => Ok((Button::B2, level)),
                3 => Ok((Button::B3, level)),
                4 => Ok((Button::B4, level)),
                unknown => panic!("Unexpected PIN value: {}", unknown),
            },
            Ok(None) => {
                panic!("Blocking call to poll_interrupts should never return None")
            }
            Err(err) => Err(err),
        }
    }
}

pub(crate) struct RealRpiOutput {
    ledpin1: OutputPin,
    ledpin2: OutputPin,
    ledpin3: OutputPin,
    ledpin4: OutputPin,
}

impl RpiOutput for RealRpiOutput {
    fn switch_led(self: &mut Self, led: Led, is_on: bool) {
        match is_on {
            true => match led {
                Led::L1 => self.ledpin1.set_high(),
                Led::L2 => self.ledpin2.set_high(),
                Led::L3 => self.ledpin3.set_high(),
                Led::L4 => self.ledpin4.set_high(),
            },
            false => match led {
                Led::L1 => self.ledpin1.set_low(),
                Led::L2 => self.ledpin2.set_low(),
                Led::L3 => self.ledpin3.set_low(),
                Led::L4 => self.ledpin4.set_low(),
            },
        }
    }
}

struct FakeRpiOutput {}

impl RpiOutput for FakeRpiOutput {
    fn switch_led(self: &mut Self, led: Led, is_on: bool) {
        info!("Switching {:?} to {}", led, is_on);
    }
}

struct FakeRpiInput {
    stdin: Stdin,
}

impl RpiInput for FakeRpiInput {
    fn poll_interrupts(
        self: &mut Self,
    ) -> Result<(Button, rppal::gpio::Level), rppal::gpio::Error> {
        let mut next: [u8; 1] = [0; 1];

        loop {
            // Bit silly to read one byte at a time, but this is just for testing and
            // we're not going to be hammering the keyboard so never mind.
            let bytes_read: usize = {
                let mut handle = self.stdin.lock();
                handle.read(&mut next)?
            };

            if bytes_read == 0 {
                panic!("Blocking read should never return 0?")
            } else {
                debug!("Read byte from stdin: {}", next[0]);
                return match next[0] {
                    49 => Ok((Button::B1, Level::High)),
                    50 => Ok((Button::B2, Level::High)),
                    51 => Ok((Button::B3, Level::High)),
                    52 => Ok((Button::B4, Level::High)),
                    // Ignore enter key
                    10 => continue,
                    113 => Ok((Button::STOP, Level::High)),
                    unknown => {
                        info!("Unknown input {}", unknown);
                        continue;
                    }
                };
            }
        }
    }
}
