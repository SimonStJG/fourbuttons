use anyhow::{Context, Result};
use std::{
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use crate::actor::actor::Actor;

use super::runner::Runner;

pub(super) struct ActorHandle<T> {
    pub(super) sender: Sender<T>,
    pub(super) join_handle: JoinHandle<Result<()>>,
}

impl<T> ActorHandle<T>
where
    T: Send + Sync + 'static,
{
    pub(super) fn new<U>(actor: U, name: String, runner: Runner) -> Result<Self>
    where
        U: Actor<T> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel::<T>();
        let join_handle = thread::Builder::new()
            .name(name)
            .spawn(move || {
                runner.run_actor(&receiver, actor)?;
                // Runner should be dropped here in order to notify supervisor
                Ok(())
            })
            .context("Failed to start actor thread")?;

        Ok(Self {
            sender,
            join_handle,
        })
    }
}
