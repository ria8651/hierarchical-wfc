use crate::wfc_task::WfcTask;
use anyhow::Result;

pub use multi_threaded::MultiThreaded;
pub use single_threaded::SingleThreaded;

pub mod multi_threaded;
pub mod single_threaded;
pub trait Backend {
    fn queue_task(&mut self, task: WfcTask) -> Result<()>;
}
