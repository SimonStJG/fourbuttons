use std::sync::mpsc::Sender;

use chrono::Local;
use log::info;

use crate::scheduler::Scheduler;

use super::{actor::Actor, control_actor::ControlActorMessage};

pub(crate) enum SchedulerActorMessage {
    Tick,
}

pub(crate) struct SchedulerActor {
    scheduler: Scheduler,
    tx_control: Sender<ControlActorMessage>,
}

impl SchedulerActor {
    pub(crate) fn new(scheduler: Scheduler, tx_control: Sender<ControlActorMessage>) -> Self {
        Self {
            scheduler,
            tx_control,
        }
    }
}

impl Actor<SchedulerActorMessage> for SchedulerActor {
    fn startup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn handle_message(&mut self, _: SchedulerActorMessage) -> anyhow::Result<bool> {
        let now = Local::now().naive_local();
        for activity in self.scheduler.tick(now) {
            info!("Activity triggered: {:?}", activity);
            self.tx_control
                .send(ControlActorMessage::Activity(activity, now))?;
        }

        Ok(false)
    }
}
