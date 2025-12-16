// Thin wrapper that selects the correct board-specific USB implementation
#[cfg(feature = "rp2040")]
pub use crate::board::rp2040::usb::*;

#[cfg(feature = "rp2350")]
pub use crate::board::rp2350::usb::*;

// If neither is selected, compile-time error will be raised by board/bsp.rs
