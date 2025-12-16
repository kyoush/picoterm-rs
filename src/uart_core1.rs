//! Minimal UART helpers for Core1
//!
//! Provides low-level UART0 access for Core1 without HAL generics.
//! Core1 reads and writes directly to UART0 registers, avoiding pin
//! reconfiguration.

use crate::pac;
use core::sync::atomic::{AtomicPtr, Ordering};

/// UART FR register RX FIFO empty bit
const RXFE_BIT: u32 = 1 << 4;
/// UART FR register TX FIFO full bit
const TXFF_BIT: u32 = 1 << 5;

/// Static pointer to UART0 peripheral, initialized once
static UART0_PTR: AtomicPtr<pac::uart0::RegisterBlock> = AtomicPtr::new(core::ptr::null_mut());

/// Initialize UART0 pointer for Core1 (called once before spawning Core1)
pub fn init_uart_ptr() {
    let uart0_addr = pac::UART0::ptr() as *mut pac::uart0::RegisterBlock;
    UART0_PTR.store(uart0_addr, Ordering::Release);
}

/// Get UART0 reference (safe after init_uart_ptr)
#[inline]
fn uart0() -> &'static pac::uart0::RegisterBlock {
    let ptr = UART0_PTR.load(Ordering::Acquire);
    debug_assert!(!ptr.is_null(), "UART0 not initialized");
    unsafe { &*ptr }
}

/// Returns true if UART0 has readable data
#[inline]
pub fn is_readable() -> bool {
    (uart0().uartfr().read().bits() & RXFE_BIT) == 0
}

/// Reads one byte from UART0 (ensure is_readable() is true first)
#[inline]
pub fn read_byte() -> u8 {
    (uart0().uartdr().read().bits() & 0xFF) as u8
}

/// Returns true if UART0 TX FIFO is full
#[inline]
pub fn is_tx_full() -> bool {
    (uart0().uartfr().read().bits() & TXFF_BIT) != 0
}

/// Writes one byte to UART0 (TX FIFO must not be full)
#[inline]
pub fn write_byte(b: u8) {
    uart0().uartdr().write(|w| unsafe { w.bits(u32::from(b)) });
}
