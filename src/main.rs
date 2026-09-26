//! Finarr, all-in-one media automation in a single binary.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // finarr::run installs the tracing subscriber (formatter + ring buffer).
    finarr::run(std::env::args_os().skip(1)).await
}
