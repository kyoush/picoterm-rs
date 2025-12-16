//! BSP (Board Support Package) abstraction for multi-target support

#[cfg(feature = "rp2040")]
pub use rp_pico::entry;

#[cfg(feature = "rp2040")]
pub use rp_pico::hal;

#[cfg(not(any(feature = "rp2040", feature = "rp2350")))]
compile_error!("No board feature selected. Enable --features rp2040 or rp2350");
