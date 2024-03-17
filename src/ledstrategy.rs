use crate::rpi::Led;
use crate::rpi::RpiOutput;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(unused)]
pub(crate) enum LedState {
    On,
    Off,
    BlinkTemporary,
}

pub(crate) trait LedStrategy {
    fn tick(&mut self, instant: Instant, rpi: &mut dyn RpiOutput);
}

pub(crate) struct LedStrategies {
    pub(crate) l1: Box<dyn LedStrategy>,
    pub(crate) l2: Box<dyn LedStrategy>,
    pub(crate) l3: Box<dyn LedStrategy>,
    pub(crate) l4: Box<dyn LedStrategy>,
}

impl LedStrategies {
    pub(crate) fn all_off(rpi: &mut dyn RpiOutput) -> LedStrategies {
        rpi.switch_led(Led::L1, false);
        rpi.switch_led(Led::L2, false);
        rpi.switch_led(Led::L3, false);
        rpi.switch_led(Led::L4, false);

        LedStrategies {
            l1: Box::new(LedStrategyOff {}),
            l2: Box::new(LedStrategyOff {}),
            l3: Box::new(LedStrategyOff {}),
            l4: Box::new(LedStrategyOff {}),
        }
    }

    pub(crate) fn tick(&mut self, instant: Instant, rpi: &mut dyn RpiOutput) {
        self.l1.tick(instant, rpi);
        self.l2.tick(instant, rpi);
        self.l3.tick(instant, rpi);
        self.l4.tick(instant, rpi);
    }

    pub(crate) fn update(&mut self, rpi: &mut dyn RpiOutput, led: Led, led_state: LedState) {
        let new_state: Box<dyn LedStrategy> = match led_state {
            LedState::On => Box::new(LedStrategyOn::new(led, &mut *rpi)),
            LedState::Off => Box::new(LedStrategyOff::new(led, &mut *rpi)),
            LedState::BlinkTemporary => Box::new(LedStrategyBlinkTemporary::new(led, &mut *rpi)),
        };
        match led {
            Led::L1 => self.l1 = new_state,
            Led::L2 => self.l2 = new_state,
            Led::L3 => self.l3 = new_state,
            Led::L4 => self.l4 = new_state,
        }
    }
}

pub(crate) struct LedStrategyOn {}

impl LedStrategyOn {
    pub(crate) fn new(led: Led, rpi: &mut dyn RpiOutput) -> LedStrategyOn {
        rpi.switch_led(led, true);
        LedStrategyOn {}
    }
}

impl LedStrategy for LedStrategyOn {
    fn tick(&mut self, _instant: Instant, _rpi: &mut dyn RpiOutput) {}
}

pub(crate) struct LedStrategyOff {}

impl LedStrategyOff {
    pub(crate) fn new(led: Led, rpi: &mut dyn RpiOutput) -> LedStrategyOff {
        rpi.switch_led(led, false);
        LedStrategyOff {}
    }
}

impl LedStrategy for LedStrategyOff {
    fn tick(&mut self, _instant: Instant, _rpi: &mut dyn RpiOutput) {}
}

pub(crate) struct LedStrategyBlinkTemporary {
    pub(crate) is_on: bool,
    pub(crate) stopped: bool,
    pub(crate) created_at: Instant,
    pub(crate) last_change: Instant,
    pub(crate) led: Led,
}

impl LedStrategyBlinkTemporary {
    pub(crate) fn new(led: Led, rpi: &mut dyn RpiOutput) -> LedStrategyBlinkTemporary {
        rpi.switch_led(led, true);
        let now = Instant::now();
        LedStrategyBlinkTemporary {
            is_on: true,
            stopped: false,
            created_at: now,
            last_change: now,
            led,
        }
    }
}

impl LedStrategy for LedStrategyBlinkTemporary {
    fn tick(&mut self, instant: Instant, rpi: &mut dyn RpiOutput) {
        if self.stopped {
            return;
        }

        if instant - self.created_at >= Duration::from_millis(1000) {
            self.stopped = true;
            rpi.switch_led(self.led, false);
        } else if instant - self.last_change >= Duration::from_millis(100) {
            self.last_change = instant;
            self.is_on = !self.is_on;
            rpi.switch_led(self.led, self.is_on);
        }
    }
}
