use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Condvar, Mutex},
    thread::JoinHandle,
};

use anyhow::Result;
use log::{debug, error, info, warn};

use crate::actor::{actor::Actor, message_source::MessageSource};

use super::{
    actor_handle::ActorHandle, message_source_handle::MessageSourceHandle, runner::Runner,
};

pub(crate) struct Supervisor {
    next_actor_id: u32,
    handles: HashMap<u32, JoinHandle<Result<()>>>,
    completed_actors: Arc<(Mutex<Vec<u32>>, Condvar)>,
}

impl Supervisor {
    pub(crate) fn new() -> Self {
        Self {
            next_actor_id: 0,
            handles: HashMap::new(),
            completed_actors: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
        }
    }

    pub(crate) fn start<T, U>(&mut self, actor: T, name: String) -> Result<Sender<U>>
    where
        T: Actor<U> + Send + 'static,
        U: Send + Sync + 'static,
    {
        let actor_id = self.get_next_actor_id();
        let runner = Runner::new(self.completed_actors.clone(), actor_id);

        let handle = ActorHandle::new(actor, name, runner)?;

        self.handles.insert(actor_id, handle.join_handle);

        Ok(handle.sender)
    }

    pub(crate) fn start_message_source<T>(&mut self, source_actor: T, name: String) -> Result<()>
    where
        T: MessageSource + Send + 'static,
    {
        let actor_id = self.get_next_actor_id();
        let runner = Runner::new(self.completed_actors.clone(), actor_id);
        let handle = MessageSourceHandle::new(source_actor, name, runner)?;
        self.handles.insert(actor_id, handle.join_handle);

        Ok(())
    }

    pub(crate) fn supervise(&mut self) {
        loop {
            if let Some(actor_id) = self.wait_for_completed_actor() {
                let should_terminate = self.handle_completed_actor(actor_id);
                if should_terminate {
                    return;
                }
            }
        }
    }

    fn handle_completed_actor(&mut self, actor_id: u32) -> bool {
        debug!("Actor ID completed {:?}", actor_id);
        match self.handles.remove(&actor_id) {
            Some(join_handle) => match join_handle.join() {
                Ok(join_result) => match join_result {
                    Ok(()) => {
                        info!("Actor clean shutdown: {:?}", actor_id);
                    }
                    Err(err) => {
                        error!("Error in actor: {:?} {:?}", actor_id, err);
                    }
                },
                Err(err) => {
                    error!("Error joining actor {:?} {:?}", actor_id, err);
                }
            },
            None => {
                // I don't think this will ever happen?
                warn!(
                    "Got actor completed notification for already completed actor {:?}",
                    actor_id
                );
            }
        }

        // Could be cleverer here, but for now let's just exit
        true
    }

    fn get_next_actor_id(&mut self) -> u32 {
        let thread_id = self.next_actor_id;
        self.next_actor_id += 1;
        thread_id
    }

    fn wait_for_completed_actor(&self) -> Option<u32> {
        let (mutex, cvar) = &*self.completed_actors;
        let mut completed_actors = cvar.wait(mutex.lock().unwrap()).unwrap();
        completed_actors.pop()
    }
}
