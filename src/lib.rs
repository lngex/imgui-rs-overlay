
pub mod window;
pub type Result<T> = windows::core::Result<T>;
pub use window_tracker::WINDOWS_RECT;
mod window_tracker;
mod d3d11;