// RP2040 固有のハードウェア設定をまとめるファイルです。
// 今は外部クリスタルの周波数とピン初期化を提供します。

pub mod usb;

/// 外部クリスタルの周波数（Hz）
pub const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000;

use super::hal as bsp_hal;
use crate::DynLedPin;
use crate::UART_BAUD_RATE;
use bsp_hal::gpio::FunctionUart;
use bsp_hal::gpio::Pins;
use bsp_hal::pac;
use bsp_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use fugit::RateExtU32;

/// Board-specific Timer alias so `main.rs` can reference the HAL's Timer type.
pub type BoardTimer = bsp_hal::timer::Timer;

/// Create a board-specific timer instance. `main.rs` calls this to obtain
/// a concrete `BoardTimer` without depending on HAL generics.
pub fn make_timer(
    pac_timer: pac::TIMER,
    resets: &mut pac::RESETS,
    clocks: &bsp_hal::clocks::ClocksManager,
) -> BoardTimer {
    bsp_hal::timer::Timer::new(pac_timer, resets, clocks)
}

/// RP2040 用にピンを初期化し、Core0 側で UART0 を有効化します。
pub fn init_pins_and_enable_uart(
    pins: Pins,
    pac_uart0: pac::UART0,
    resets: &mut pac::RESETS,
    system_freq_hz: u32,
) -> Result<DynLedPin, &'static str> {
    // UART ピンを設定
    let uart_tx = pins.gpio0.into_function::<FunctionUart>();
    let uart_rx = pins.gpio1.into_function::<FunctionUart>();

    let uart_config = UartConfig::new(UART_BAUD_RATE.Hz(), DataBits::Eight, None, StopBits::One);
    match UartPeripheral::new(pac_uart0, (uart_tx, uart_rx), resets)
        .enable(uart_config, system_freq_hz.Hz())
    {
        Ok(u) => {
            let _u = u; // ローカルに束縛して所有権を移動させない
        }
        Err(_) => return Err("Failed to enable UART0 via HAL"),
    }

    // LED ピンを取得して返す
    let led_local = pins.gpio25.into_push_pull_output().into_dyn_pin();
    Ok(led_local)
}
