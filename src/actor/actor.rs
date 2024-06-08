use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result};

pub(crate) trait Actor<T> {
    fn startup(&mut self) -> Result<()>;
    fn handle_message(&mut self, msg: T) -> Result<()>;
}

// TODO Get rid of this, it doesn't actually serve any purpose!
struct ActorWrapper<T, U> {
    receiver: Receiver<T>,
    actor: U,
}

impl<T, U> ActorWrapper<T, U> {
    fn new(receiver: Receiver<T>, actor: U) -> Self {
        ActorWrapper { receiver, actor }
    }
}

pub(crate) struct ActorHandle<T> {
    pub(crate) sender: Sender<T>,
    pub(crate) join_handle: JoinHandle<Result<()>>,
}

impl<T> ActorHandle<T>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new<U>(state: U) -> Result<Self>
    where
        U: Actor<T> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel::<T>();
        let actor = ActorWrapper::new(receiver, state);
        let join_handle = thread::Builder::new()
            .name("test".to_string())
            .spawn(move || {
                run_actor(actor)?;
                Ok(())
            })
            .context("Failed to start actor thread")?;

        Ok(Self {
            sender,
            join_handle,
        })
    }
}

fn run_actor<T, U>(mut actor_wrapper: ActorWrapper<T, U>) -> Result<()>
where
    U: Actor<T>,
{
    actor_wrapper
        .actor
        .startup()
        .context("Error in actor startup")?;
    while let Ok(msg) = actor_wrapper.receiver.recv() {
        actor_wrapper
            .actor
            .handle_message(msg)
            .context("Error handling actor message")?;
    }

    Ok(())
}
