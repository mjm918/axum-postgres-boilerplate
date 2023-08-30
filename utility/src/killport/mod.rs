#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Clone, Copy, Debug)]
pub enum KillPortSignalOptions {
	SIGKILL,
	SIGTERM,
}

#[cfg(target_os = "linux")]
pub use linux::kill_processes_by_port;
#[cfg(target_os = "macos")]
pub use macos::kill_processes_by_port;
#[cfg(target_os = "windows")]
pub use windows::kill_processes_by_port;