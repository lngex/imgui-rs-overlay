pub mod window;
pub type Result<T> = windows::core::Result<T>;
pub use window_tracker::OverlayTarget;
pub use window_tracker::WINDOWS_RECT;
mod d3d11;
mod window_tracker;
