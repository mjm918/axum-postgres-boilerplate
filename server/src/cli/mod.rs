use clap::{Parser, Subcommand};
use utility::errors::AppResult;

#[derive(Parser)]
#[clap(
name = crate::APP_NAME,
version = clap::crate_version ! (),
author = clap::crate_authors ! (),
)]
struct Cli {
	#[clap(subcommand)]
	commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Start server
	#[clap(about = "Start server", long_about = None)]
	Serve,
}

pub async fn start() -> AppResult<()> {
	let args = Cli::parse();
	match &args.commands {
		Commands::Serve => crate::server::serve().await
	}
}