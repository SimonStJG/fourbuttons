use rppal::gpio::{Gpio, InputPin, OutputPin};
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    error::Error,
    fs,
    io::{self, Read, Stdin},
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

const PIN_BUTTON_1: u8 = 2;
const PIN_BUTTON_2: u8 = 3;
const PIN_BUTTON_3: u8 = 20;
const PIN_BUTTON_4: u8 = 21;
const PIN_LED_1: u8 = 23;
const PIN_LED_2: u8 = 24;
const PIN_LED_3: u8 = 22;
const PIN_LED_4: u8 = 27;

// This does look ridiculously high, but I've seen bounces into the hundreds
// of ms on these switches quite regularly, and I don't need to worry about
// quick succession button presses for this machine.
const DEBOUNCE_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Button {
    B1,
    B2,
    B3,
    B4,
    // Special button to stop the app
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
    fn wait_for_button_press(self: &mut Self) -> Result<Button, rppal::gpio::Error>;
}

pub(crate) trait RpiOutput {
    fn switch_led(self: &mut Self, led: Led, is_on: bool);
}

// We need +Send because this is going to be shared between threads later when used for I/O
pub(crate) fn initialise_rpi(
) -> Result<(Box<dyn RpiInput + Send>, Box<dyn RpiOutput + Send>), Box<dyn Error>> {
    if !env::var("USE_FAKE_RPI").is_ok() {
        debug!("Initialising RPi");

        let gpio = new_rpi().unwrap();

        let mut btnpin1 = gpio.get(PIN_BUTTON_1)?.into_input_pullup();
        let mut btnpin2 = gpio.get(PIN_BUTTON_2)?.into_input_pullup();
        let mut btnpin3 = gpio.get(PIN_BUTTON_3)?.into_input_pullup();
        let mut btnpin4 = gpio.get(PIN_BUTTON_4)?.into_input_pullup();

        let ledpin1 = gpio.get(PIN_LED_1)?.into_output_low();
        let ledpin2 = gpio.get(PIN_LED_2)?.into_output_low();
        let ledpin3 = gpio.get(PIN_LED_3)?.into_output_low();
        let ledpin4 = gpio.get(PIN_LED_4)?.into_output_low();

        btnpin1.set_interrupt(rppal::gpio::Trigger::FallingEdge)?;
        btnpin2.set_interrupt(rppal::gpio::Trigger::FallingEdge)?;
        btnpin3.set_interrupt(rppal::gpio::Trigger::FallingEdge)?;
        btnpin4.set_interrupt(rppal::gpio::Trigger::FallingEdge)?;

        return Ok((
            Box::new(RealRpiInput {
                gpio,
                pin1: btnpin1,
                pin2: btnpin2,
                pin3: btnpin3,
                pin4: btnpin4,
                last_trigger_1: Instant::now(),
                last_trigger_2: Instant::now(),
                last_trigger_3: Instant::now(),
                last_trigger_4: Instant::now(),
            }),
            Box::new(RealRpiOutput {
                ledpin1,
                ledpin2,
                ledpin3,
                ledpin4,
            }),
        ));
    } else {
        info!("Using fake RPi");
        Ok((
            Box::new(FakeRpiInput { stdin: io::stdin() }),
            Box::new(FakeRpiOutput {}),
        ))
    }
}

fn new_rpi() -> Result<Gpio, rppal::gpio::Error> {
    match rppal::gpio::Gpio::new() {
        Ok(gpio) => Ok(gpio),
        Err(rppal::gpio::Error::PermissionDenied(err)) => {
            warn!(
                "Permission denied on startup, trying to wait for /dev/gpiomem: {}",
                err
            );
            wait_for_gpiomem();
            info!("Found /dev/gpiomem, continuing");
            rppal::gpio::Gpio::new()
        }
        Err(err) => Err(err),
    }
}

// When starting as a system service, fourbuttons dies with a
// PemissionDenied("/dev/mem"), but I can see that actually it's
// failing to find /dev/gpiomem first.  Can't seem to find the right
// thing to wait for with systemd, so bodge it by waiting up to 5s
// for /dev/gpiomem to appear with the right permissions instead.
fn wait_for_gpiomem() {
    let gpio_mem = Path::new("/dev/gpiomem");
    for _ in 1..10 {
        match fs::metadata(gpio_mem) {
            Ok(metadata) => {
                // Even just checking for existence isn't enough,
                // because it appears first with 0600 perms and
                // then gets 0660 perms later.
                if metadata.permissions().mode() & 0o060 != 0 {
                    return;
                }
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(500));
    }
    // TODO Should return a real err type
    panic!("Can't find /dev/gpiomem even after waiting")
}

struct RealRpiInput {
    gpio: Gpio,
    pin1: InputPin,
    pin2: InputPin,
    pin3: InputPin,
    pin4: InputPin,
    last_trigger_1: Instant,
    last_trigger_2: Instant,
    last_trigger_3: Instant,
    last_trigger_4: Instant,
}

fn debounce(button: Button, last_trigger: &mut Instant) -> Option<Button> {
    let now = Instant::now();
    let gap = now - *last_trigger;
    debug!("Debouncer saw {:?} at {:?} (gap {:?})", button, now, gap);
    if gap >= DEBOUNCE_DELAY {
        *last_trigger = now;
        return Some(button);
    } else {
        return None;
    }
}

impl RpiInput for RealRpiInput {
    fn wait_for_button_press(self: &mut Self) -> Result<Button, rppal::gpio::Error> {
        loop {
            match self.gpio.poll_interrupts(
                &[&self.pin1, &self.pin2, &self.pin3, &self.pin4],
                // Setting `reset` to `false` returns any cached interrupt trigger events if available.
                false,
                None,
            ) {
                Ok(Some((pin, _))) => {
                    let trigger = match pin.pin() {
                        PIN_BUTTON_1 => debounce(Button::B1, &mut self.last_trigger_1),
                        PIN_BUTTON_2 => debounce(Button::B2, &mut self.last_trigger_2),
                        PIN_BUTTON_3 => debounce(Button::B3, &mut self.last_trigger_3),
                        PIN_BUTTON_4 => debounce(Button::B4, &mut self.last_trigger_4),
                        unknown => panic!("Unexpected PIN value: {}", unknown),
                    };
                    match trigger {
                        Some(button) => return Ok(button),
                        None => continue,
                    }
                }
                Ok(None) => {
                    panic!("Blocking call to poll_interrupts should never return None")
                }
                Err(err) => return Err(err),
            }
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
    fn wait_for_button_press(self: &mut Self) -> Result<Button, rppal::gpio::Error> {
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
                    49 => Ok(Button::B1),
                    50 => Ok(Button::B2),
                    51 => Ok(Button::B3),
                    52 => Ok(Button::B4),
                    // Ignore enter key
                    10 => continue,
                    113 => Ok(Button::STOP),
                    unknown => {
                        info!("Unknown input {}", unknown);
                        continue;
                    }
                };
            }
        }
    }
}
