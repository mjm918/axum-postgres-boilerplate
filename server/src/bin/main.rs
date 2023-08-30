use utility::errors::AppResult;

#[tokio::main]
async fn main() -> AppResult<()> {
	server::cli::start().await
}