use crate::{
    actor::actor::Actor,
    ledstrategy::{self, LedState, LedStrategies},
    rpi::{Led, RpiOutput},
};
use anyhow::Result;
use std::time::Instant;

#[derive(PartialEq, PartialOrd, Debug)]
pub(crate) enum LedActorMessage {
    Tick(Instant),
    StateChange { led: Led, state: LedState },
}

pub(crate) struct LedActor {
    rpi: Box<dyn RpiOutput + Send>,
    strategies: LedStrategies,
}

impl LedActor {
    pub(crate) fn new(mut rpi: Box<dyn RpiOutput + Send>) -> LedActor {
        let strategies = ledstrategy::LedStrategies::all_off(&mut *rpi);
        Self { rpi, strategies }
    }
}

impl Actor<LedActorMessage> for LedActor {
    fn handle_message(&mut self, msg: LedActorMessage) -> anyhow::Result<bool> {
        match msg {
            LedActorMessage::Tick(instant) => {
                self.strategies.tick(instant, &mut *self.rpi);
            }
            LedActorMessage::StateChange { led, state } => {
                self.strategies.update(&mut *self.rpi, led, state);
            }
        };

        Ok(false)
    }

    fn startup(&mut self) -> Result<()> {
        Ok(())
    }
}
