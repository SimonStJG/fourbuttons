use anyhow::{Context, Result};
use log::{debug, info};
use std::sync::{mpsc::Receiver, Arc, Condvar, Mutex};

use crate::actor::{actor::Actor, message_source::MessageSource};

/// Notifies `completed_actors` with the `actor_id` when it goes out of scope
pub(crate) struct Runner {
    // Supervisor's list of completed actors
    completed_actors: Arc<(Mutex<Vec<u32>>, Condvar)>,
    // This actors actor_id
    actor_id: u32,
}

impl Runner {
    pub(crate) fn new(completed_actors: Arc<(Mutex<Vec<u32>>, Condvar)>, actor_id: u32) -> Self {
        Self {
            completed_actors,
            actor_id,
        }
    }

    pub(crate) fn run_actor<T, U>(&self, receiver: &Receiver<T>, mut actor: U) -> Result<()>
    where
        U: Actor<T>,
    {
        debug!("Running Actor: {}", self.actor_id);
        actor.startup().context("Error in actor startup")?;
        while let Ok(msg) = receiver.recv() {
            let should_terminate = actor
                .handle_message(msg)
                .context("Error handling actor message")?;

            if should_terminate {
                return Ok(());
            }
        }

        Ok(())
    }

    pub(crate) fn run_message_source<T>(&self, mut message_source: T) -> Result<()>
    where
        T: MessageSource,
    {
        debug!("Running MsgSource: {}", self.actor_id);
        loop {
            let should_terminate = message_source
                .run()
                .context("Error on MessageSource `run`")?;

            if should_terminate {
                return Ok(());
            }
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        info!("Drop Actor: {}", self.actor_id);
        let (mutex, cvar) = &*self.completed_actors;
        mutex.lock().unwrap().push(self.actor_id);
        cvar.notify_one();
    }
}
