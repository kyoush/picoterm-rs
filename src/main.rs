#![no_std]
#![no_main]

mod uart_core1;

use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;
use rp_pico::entry;
use rp_pico::hal::{
    clocks::init_clocks_and_plls,
    gpio::FunctionUart,
    multicore::{Multicore, Stack},
    pac,
    uart::{DataBits, StopBits, UartConfig, UartPeripheral},
    usb::UsbBus,
    watchdog::Watchdog,
    Sio,
};
use usb_device::prelude::*;
use usbd_serial::SerialPort;

const XTAL_FREQ_HZ: u32 = 12_000_000;
const UART_BAUD_RATE: u32 = 115_200;
const CORE1_STACK_SIZE: usize = 4096;

static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;
static mut CORE1_STACK: Stack<CORE1_STACK_SIZE> = Stack::new();

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

    let mut sio = Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // Initialize UART
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

    // Initialize USB
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    unsafe {
        USB_BUS = Some(usb_bus);
    }

    let mut serial = SerialPort::new(unsafe { USB_BUS.as_ref().unwrap() });

    let mut usb_dev = UsbDeviceBuilder::new(
        unsafe { USB_BUS.as_ref().unwrap() },
        UsbVidPid(0x16c0, 0x27dd),
    )
    .strings(&[StringDescriptors::default()
        .manufacturer("Raspberry Pi")
        .product("PicoTerm")
        .serial_number("12345678")])
    .unwrap()
    .device_class(2)
    .build();

    // Start Core1
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    core1
        .spawn(unsafe { &mut CORE1_STACK.mem }, move || {
            uart_core1::uart_core1_task()
        })
        .unwrap();

    defmt::info!("Dual-core bridge started (bidirectional via FIFO)");

    // Core0: Handle USB bidirectionally with Core1
    loop {
        if usb_dev.poll(&mut [&mut serial]) {
            let mut buf = [0u8; 64];
            
            // USB -> FIFO -> Core1 -> UART
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    for &byte in &buf[..count] {
                        sio.fifo.write(byte as u32);
                    }
                }
                _ => {}
            }
        }

        // UART -> Core1 -> FIFO -> USB
        if let Some(data) = sio.fifo.read() {
            let byte = (data & 0xFF) as u8;
            let _ = serial.write(&[byte]);
        }

        cortex_m::asm::nop();
    }
}
