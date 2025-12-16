#![no_std]
#![no_main]

use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;
use rp_pico::entry;
use rp_pico::hal::{
    clocks::init_clocks_and_plls,
    gpio::FunctionUart,
    pac,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    watchdog::Watchdog,
    Sio,
};

const XTAL_FREQ_HZ: u32 = 12_000_000;
const UART_BAUD_RATE: u32 = 115_200;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let _core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Initialize UART0 on pins GPIO0 (TX) and GPIO1 (RX)
    let uart_pins = (
        pins.gpio0.into_function::<FunctionUart>(),
        pins.gpio1.into_function::<FunctionUart>(),
    );

    let _uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(UART_BAUD_RATE.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    defmt::info!("UART initialized at {} baud", UART_BAUD_RATE);

    loop {
        cortex_m::asm::nop();
    }
}
