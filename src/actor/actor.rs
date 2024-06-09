use anyhow::Result;

pub(crate) trait Actor<T> {
    fn startup(&mut self) -> Result<()>;
    fn handle_message(&mut self, msg: T) -> Result<bool>;
}
