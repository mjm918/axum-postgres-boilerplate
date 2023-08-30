use std::sync::{Arc, LockResult, Mutex};
use std::time::Duration;
use flinch::database::Database;
use flinch::doc::QueryBased;
use tokio::signal;
use tracing::{error, info};
use db::setup::PgServer;
use crate::{APP_NAME, GENERAL_BUCKET, RATE_LIMITER_BUCKET};

pub async fn get_flinch() -> Arc<Database<QueryBased>> {
	let mem = Database::<QueryBased>::init_with_name(APP_NAME).await;
	let options = |name: &str| {
		flinch::database::CollectionOptions {
			name: name.to_string(),
			index_opts: vec![],
			search_opts: vec![],
			view_opts: vec![],
			range_opts: vec![],
			clips_opts: vec![],
		}
	};
	let _ = mem.add(options(RATE_LIMITER_BUCKET)).await;
	let _ = mem.add(options(GENERAL_BUCKET)).await;

	Arc::new(mem)
}

#[allow(unused)]
pub async fn graceful_shutdown(handle: axum_server::Handle, mut pg_server: Arc<Mutex<PgServer>>) {
	// Stop postgres first
	match pg_server.lock() {
		Ok(mut server) => {
			server.stop_db_sync();
		}
		Err(err) => {
			error!("{:?}",err);
		}
	}

	let ctrl_c = async {
		signal::ctrl_c().await.expect("failed to install Ctrl+C handlers");
	};

	#[cfg(unix)]
		let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("failed to install signal handlers")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
		let terminate = std::future::pending::<()>();

	tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
	handle.graceful_shutdown(Some(Duration::from_secs(5)));
	info!("signal received, starting graceful shutdown");
}

#[allow(unused)]
pub async fn shutdown_signal(pg_server: Arc<Mutex<PgServer>>) {
	// Stop postgres first
	match pg_server.lock() {
		Ok(mut server) => {
			server.stop_db_sync();
		}
		Err(err) => {
			error!("{:?}",err);
		}
	}

	let ctrl_c = async {
		signal::ctrl_c().await.expect("failed to install Ctrl+C handlers");
	};

	#[cfg(unix)]
		let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("failed to install signal handlers")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
		let terminate = std::future::pending::<()>();

	tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

	info!("signal received, starting graceful shutdown");
}