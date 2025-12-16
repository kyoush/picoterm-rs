// RP2350 board scaffold

pub mod usb;

/// External crystal frequency for RP2350 boards (Hz)
pub const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000;

use super::hal as bsp_hal;
use crate::DynLedPin;
use crate::UART_BAUD_RATE;
use bsp_hal::gpio::FunctionUart;
use bsp_hal::gpio::Pins;
use bsp_hal::pac;
use bsp_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use fugit::RateExtU32;

// Export a concrete timer type so `main.rs` can refer to `board::BoardTimer`.
pub type BoardTimer<D> = bsp_hal::timer::Timer<D>;
use bsp_hal::timer::CopyableTimer0;

/// Construct the board timer (Timer0 specialized)
pub fn make_timer(
    pac_timer: pac::TIMER0,
    resets: &mut pac::RESETS,
    clocks: &bsp_hal::clocks::ClocksManager,
) -> BoardTimer<CopyableTimer0> {
    // Use the rp235x-hal helper which returns Timer<CopyableTimer0>
    bsp_hal::timer::Timer::new_timer0(pac_timer, resets, clocks)
}

/// Initialize pins and enable UART0. This is a stub that assumes
/// rp235x-hal provides similar APIs to rp2040-hal; adjust when testing on hardware.
pub fn init_pins_and_enable_uart(
    pins: Pins,
    pac_uart0: pac::UART0,
    resets: &mut pac::RESETS,
    system_freq_hz: u32,
) -> Result<DynLedPin, &'static str> {
    let uart_tx = pins.gpio0.into_function::<FunctionUart>();
    let uart_rx = pins.gpio1.into_function::<FunctionUart>();

    let uart_config = UartConfig::new(UART_BAUD_RATE.Hz(), DataBits::Eight, None, StopBits::One);
    match UartPeripheral::new(pac_uart0, (uart_tx, uart_rx), resets)
        .enable(uart_config, system_freq_hz.Hz())
    {
        Ok(_u) => {}
        Err(_) => return Err("Failed to enable UART0 via rp235x-hal"),
    }

    // Placeholder LED pin — change to actual board LED pin if known
    let led_local = pins.gpio25.into_push_pull_output().into_dyn_pin();
    Ok(led_local)
}
