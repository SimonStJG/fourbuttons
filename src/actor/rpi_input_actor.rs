use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use log::debug;

use crate::{actor::message_source::MessageSource, rpi::RpiInput};

use super::control_actor::ControlActorMessage;

pub(crate) struct RpiInputActor {
    rpi: Box<dyn RpiInput + Send>,
    tx: Sender<ControlActorMessage>,
}

impl RpiInputActor {
    pub(crate) fn new(rpi: Box<dyn RpiInput + Send>, tx: Sender<ControlActorMessage>) -> Self {
        Self { rpi, tx }
    }
}

impl MessageSource for RpiInputActor {
    fn run(&mut self) -> Result<bool> {
        let button = self
            .rpi
            .wait_for_button_press()
            .context("RPI Input Actor failed to wait for button press")?;
        debug!("Sending: {:?}", button);

        self.tx
            .send(ControlActorMessage::ButtonPress(button))
            .context("RPI Input Actor failed to send to tx")?;

        Ok(false)
    }
}
