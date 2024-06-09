use std::{
    sync::mpsc::Sender,
    thread,
    time::{Duration, Instant},
};

use crate::actor::message_source::MessageSource;

pub(crate) struct TickActor<T> {
    duration: Duration,
    tx: Sender<T>,
    message_builder: fn(Instant) -> T,
}

impl<T> TickActor<T> {
    pub(crate) fn new(
        duration: Duration,
        tx: Sender<T>,
        message_builder: fn(Instant) -> T,
    ) -> Self {
        Self {
            duration,
            tx,
            message_builder,
        }
    }
}

impl<T> MessageSource for TickActor<T>
where
    T: Send + Sync + 'static,
{
    fn run(&mut self) -> anyhow::Result<bool> {
        thread::sleep(self.duration);
        self.tx.send((self.message_builder)(Instant::now()))?;

        Ok(false)
    }
}
