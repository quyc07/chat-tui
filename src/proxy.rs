pub mod friend;
pub mod group;
pub mod user;

use color_eyre::eyre::format_err;
use tokio::task::spawn_blocking;

pub(crate) static HOST: &str = include_str!("../host");

pub(crate) fn send_request<F, R>(f: F) -> color_eyre::Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let join_handle = spawn_blocking(f);
    futures::executor::block_on(join_handle).map_err(|e| format_err!("failed to send request:{e}"))
}
