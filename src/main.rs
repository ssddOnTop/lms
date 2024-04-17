use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    lms::cli::runner::fork_run().await?; // TODO improve error handling
    Ok(())
}
