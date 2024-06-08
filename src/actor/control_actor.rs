use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use log::{error, info};

use crate::{
    activity,
    appdb::AppDb,
    application_state::ApplicationState,
    email::Email,
    ledstrategy::LedState,
    rpi::{Button, Led},
    Activity,
};

use super::{actor::Actor, led_actor::LedActorMessage};

pub(crate) enum ControlActorMessage {
    Activity(Activity, NaiveDateTime),
    ButtonPress(Button),
}

pub(crate) struct ControlActor {
    tx_led: Sender<LedActorMessage>,
    application_state: ApplicationState,
    db: AppDb,
    email: Email,
}

impl ControlActor {
    pub(crate) fn new(
        tx_led: Sender<LedActorMessage>,
        application_state: ApplicationState,
        db: AppDb,
        email: Email,
    ) -> Self {
        Self {
            tx_led,
            application_state,
            db,
            email,
        }
    }

    fn handle_activity(&mut self, activity: Activity, now: NaiveDateTime) -> Result<()> {
        match activity {
            activity::Activity::TakePills => {
                self.application_state.take_pills_pending = Some(now);
                self.send_led_state_change(Led::L1, LedState::On)?;
                self.db
                    .update_application_state(&self.application_state)
                    .context("Failed to update application state")?;
            }
            activity::Activity::WaterPlants => {
                self.application_state.water_plants_pending = Some(now);
                self.send_led_state_change(Led::L4, LedState::On)?;
                self.db
                    .update_application_state(&self.application_state)
                    .context("Failed to update application state")?;
            }
            activity::Activity::I => {
                self.application_state.i_pending = Some(now);
                self.send_led_state_change(Led::L3, LedState::On)?;
                self.db
                    .update_application_state(&self.application_state)
                    .context("Failed to update application state")?;
            }
            activity::Activity::TakePillsReminder => {
                if self.application_state.take_pills_pending.is_some() {
                    // It's still pending!  Time to complain further
                    if let Err(err) = self
                        .email
                        .send("Did you forget to take your pills you fool")
                    {
                        error!("Failed to send email {:?}", err);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_button_press(&mut self, button: Button) -> Result<()> {
        info!("Saw button press {:?}", button);
        // Whichever button is pressed, flash it
        // Sent any pending application state to not pending
        let led = match button {
            Button::B1 => Led::L1,
            Button::B2 => Led::L2,
            Button::B3 => Led::L3,
            Button::B4 => Led::L4,
            Button::Stop => {
                todo!("TODO Handle stop");
            }
        };

        // Important to do this first otherwise it feels laggy
        // (the db.insert_reading function called later is
        // blocking).
        self.send_led_state_change(led, LedState::BlinkTemporary)?;

        match button {
            Button::B1 => {
                self.application_state.take_pills_pending = None;
            }
            Button::B2 | Button::Stop => {}
            Button::B3 => {
                self.application_state.i_pending = None;
            }
            Button::B4 => {
                self.application_state.water_plants_pending = None;
            }
        };

        self.db
            .update_application_state(&self.application_state)
            .context("Failed to update application state")?;

        Ok(())
    }

    fn send_led_state_change(&self, led: Led, state: LedState) -> Result<()> {
        self.tx_led
            .send(LedActorMessage::StateChange { led, state })
            .context("Failed to send LedStateChange to tx_led")?;

        Ok(())
    }
}

impl Actor<ControlActorMessage> for ControlActor {
    fn startup(&mut self) -> anyhow::Result<()> {
        if self.application_state.take_pills_pending.is_some() {
            self.send_led_state_change(Led::L1, LedState::On)?;
        }
        if self.application_state.water_plants_pending.is_some() {
            self.send_led_state_change(Led::L4, LedState::On)?;
        }
        if self.application_state.i_pending.is_some() {
            self.send_led_state_change(Led::L3, LedState::On)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, msg: ControlActorMessage) -> anyhow::Result<()> {
        match msg {
            ControlActorMessage::Activity(activity, now) => self.handle_activity(activity, now),
            ControlActorMessage::ButtonPress(button) => self.handle_button_press(button),
        }
    }
}
