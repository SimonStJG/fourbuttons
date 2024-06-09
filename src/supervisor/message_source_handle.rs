use anyhow::{Context, Result};
use std::thread::{self, JoinHandle};

use crate::actor::message_source::MessageSource;

use super::runner::Runner;

pub(super) struct MessageSourceHandle {
    pub(super) join_handle: JoinHandle<Result<()>>,
}

impl MessageSourceHandle {
    pub(super) fn new<T>(message_source: T, name: String, runner: Runner) -> Result<Self>
    where
        T: MessageSource + Send + 'static,
    {
        let join_handle = thread::Builder::new()
            .name(name)
            .spawn(move || {
                runner.run_message_source(message_source)?;
                // Runner should be dropped here in order to notify supervisor
                Ok(())
            })
            .context("Failed to start source actor thread")?;

        Ok(Self { join_handle })
    }
}
