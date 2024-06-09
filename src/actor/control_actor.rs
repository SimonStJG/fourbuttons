use std::sync::mpsc::Sender;

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use log::{error, info};

use crate::{
    activity,
    appdb::AppDb,
    application_state::ApplicationState,
    email::Emailer,
    ledstrategy::LedState,
    rpi::{Button, Led},
    Activity,
};

use super::{actor::Actor, led_actor::LedActorMessage};

pub(crate) enum ControlActorMessage {
    Activity(Activity, NaiveDateTime),
    ButtonPress(Button),
}

pub(crate) struct ControlActor<TEmail>
where
    TEmail: Emailer,
{
    tx_led: Sender<LedActorMessage>,
    application_state: ApplicationState,
    db: AppDb,
    email: TEmail,
}

impl<TEmail> ControlActor<TEmail>
where
    TEmail: Emailer,
{
    pub(crate) fn new(
        tx_led: Sender<LedActorMessage>,
        application_state: ApplicationState,
        db: AppDb,
        email: TEmail,
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

    fn handle_button_press(&mut self, button: Button) -> Result<bool> {
        info!("Saw button press {:?}", button);
        // Whichever button is pressed, flash it
        // Sent any pending application state to not pending
        let led = match button {
            Button::B1 => Led::L1,
            Button::B2 => Led::L2,
            Button::B3 => Led::L3,
            Button::B4 => Led::L4,
            Button::Stop => return Ok(true),
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

        Ok(false)
    }

    fn send_led_state_change(&self, led: Led, state: LedState) -> Result<()> {
        self.tx_led
            .send(LedActorMessage::StateChange { led, state })
            .context("Failed to send LedStateChange to tx_led")?;

        Ok(())
    }
}

impl<TEmail> Actor<ControlActorMessage> for ControlActor<TEmail>
where
    TEmail: Emailer,
{
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

    fn handle_message(&mut self, msg: ControlActorMessage) -> anyhow::Result<bool> {
        match msg {
            ControlActorMessage::Activity(activity, now) => {
                self.handle_activity(activity, now)?;
                Ok(false)
            }
            ControlActorMessage::ButtonPress(button) => self.handle_button_press(button),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        str::FromStr,
        sync::mpsc::{self, Receiver, TryRecvError},
        time::Duration,
    };

    use chrono::NaiveDateTime;

    use crate::{
        actor::{actor::Actor, control_actor::ControlActorMessage, led_actor::LedActorMessage},
        appdb::AppDb,
        application_state::ApplicationState,
        email::Emailer,
        ledstrategy::LedState,
        rpi::{Button, Led},
    };

    use super::ControlActor;

    struct FakeEmail {}

    impl Emailer for FakeEmail {
        fn send(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn control_actor() -> (ControlActor<FakeEmail>, mpsc::Receiver<LedActorMessage>) {
        let (tx_led, rx_led) = mpsc::channel::<LedActorMessage>();
        let application_state = ApplicationState::blank();
        let db = AppDb::new_tmp();
        db.run_migrations().unwrap();
        let email = FakeEmail {};

        (
            ControlActor::new(tx_led, application_state, db, email),
            rx_led,
        )
    }

    fn expect_messages(
        rx_led: &Receiver<LedActorMessage>,
        num_messages: u32,
    ) -> Vec<LedActorMessage> {
        let mut messages = Vec::new();
        for _ in 0..num_messages {
            messages.push(rx_led.recv_timeout(Duration::from_millis(10)).unwrap());
        }
        assert_eq!(rx_led.try_recv(), Err(TryRecvError::Empty));

        messages
    }

    #[test]
    fn test_take_pills_activity() {
        let (mut actor, rx_led) = control_actor();

        let now = NaiveDateTime::from_str("2020-01-01T09:00:00").unwrap();
        actor
            .handle_message(ControlActorMessage::Activity(
                crate::activity::Activity::TakePills,
                now,
            ))
            .unwrap();

        assert_eq!(
            expect_messages(&rx_led, 1),
            vec![LedActorMessage::StateChange {
                led: Led::L1,
                state: LedState::On
            }]
        );

        assert_eq!(
            actor.db.load_application_state().unwrap(),
            Some(ApplicationState {
                take_pills_pending: Some(now),
                water_plants_pending: None,
                i_pending: None
            })
        );
    }

    #[test]
    fn test_take_pills_resolution() {
        let (mut actor, rx_led) = control_actor();

        let now = NaiveDateTime::from_str("2020-01-01T09:00:00").unwrap();
        actor
            .handle_message(ControlActorMessage::Activity(
                crate::activity::Activity::TakePills,
                now,
            ))
            .unwrap();
        actor
            .handle_message(ControlActorMessage::ButtonPress(Button::B1))
            .unwrap();

        assert_eq!(
            expect_messages(&rx_led, 2),
            vec![
                LedActorMessage::StateChange {
                    led: Led::L1,
                    state: LedState::On
                },
                LedActorMessage::StateChange {
                    led: Led::L1,
                    state: LedState::BlinkTemporary
                }
            ]
        );

        assert_eq!(
            actor.db.load_application_state().unwrap(),
            Some(ApplicationState::blank())
        );
    }
}
