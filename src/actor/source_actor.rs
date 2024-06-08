use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

// TODO Rename?  MessageSource?  Actors only really respond to messages, which this doesn't..
pub(crate) trait SourceActor {
    fn tick(&mut self) -> Result<()>;
}

// TODO Get rid of this, it doesn't actually serve any purpose!
struct SourceActorWrapper<T> {
    source_actor: T,
}

impl<T> SourceActorWrapper<T> {
    fn new(source_actor: T) -> Self {
        SourceActorWrapper { source_actor }
    }
}

fn run_source_actor<T>(mut actor_wrapper: SourceActorWrapper<T>) -> Result<()>
where
    T: SourceActor,
{
    loop {
        actor_wrapper
            .source_actor
            .tick()
            .context("Error handling SourceActor tick")?;
    }
}

pub struct SourceActorHandle {
    pub(crate) join_handle: JoinHandle<Result<()>>,
}

impl SourceActorHandle {
    pub fn new<T>(state: T) -> Result<Self>
    where
        T: SourceActor + Send + 'static,
    {
        let actor = SourceActorWrapper::new(state);
        let join_handle = thread::Builder::new()
            .name("test".to_string())
            .spawn(move || {
                run_source_actor(actor)?;
                Ok(())
            })
            .context("Failed to start source actor thread")?;

        Ok(Self { join_handle })
    }
}
