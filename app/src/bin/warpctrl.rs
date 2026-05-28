fn main() -> anyhow::Result<()> {
    warp_cli::local_control::run_from_env()
}
