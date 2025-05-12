
pub mod window;
pub type Result<T> = windows::core::Result<T>;
pub use window_tracker::WINDOWS_RECT;
pub use window_tracker::OverlayTarget;
mod window_tracker;
mod d3d11;