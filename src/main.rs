mod cache;
mod cli;
mod display;
mod finance;
mod portfolio;

#[tokio::main]
async fn main() {
    if let Err(err) = cli::run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
