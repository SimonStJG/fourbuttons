use anyhow::Result;

pub(crate) trait MessageSource {
    fn run(&mut self) -> Result<bool>;
}
