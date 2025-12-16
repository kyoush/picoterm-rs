#![no_std]
#![no_main]

//! Dual-core firmware: Core0 handles USB, Core1 handles UART.
//! Communication between cores uses lock-free SPSC FIFOs.

mod board;
mod uart_core1;
mod usb_serial;

use board::EXTERNAL_XTAL_FREQ_HZ;
use board::hal as bsp_hal;
use embedded_hal::digital::v2::OutputPin;
const UART_BAUD_RATE: u32 = 115_200;
const FIFO_BUFFER_SIZE: usize = 16384;
const CORE1_STACK_SIZE: usize = 1024;

use board::entry;
use defmt_rtt as _;
use panic_probe as _;

use bsp_hal::{
    clocks::{Clock, init_clocks_and_plls},
    gpio::{DynPinId, FunctionSio, Pin, Pins, PullDown, SioOutput},
    multicore::Multicore,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

type DynLedPin = Pin<DynPinId, FunctionSio<SioOutput>, PullDown>;

use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::interrupt::Mutex;
use heapless::spsc::{Consumer, Producer, Queue};

type AppResult<T> = Result<T, &'static str>;

static mut CDC_TO_UART_QUEUE: Queue<u8, FIFO_BUFFER_SIZE> = Queue::new();
static CDC_TO_UART_PRODUCER: Mutex<RefCell<Option<Producer<u8, FIFO_BUFFER_SIZE>>>> =
    Mutex::new(RefCell::new(None));
static CDC_TO_UART_CONSUMER: Mutex<RefCell<Option<Consumer<'static, u8, FIFO_BUFFER_SIZE>>>> =
    Mutex::new(RefCell::new(None));

static mut UART_TO_CDC_QUEUE: Queue<u8, FIFO_BUFFER_SIZE> = Queue::new();
static UART_TO_CDC_PRODUCER: Mutex<RefCell<Option<Producer<u8, FIFO_BUFFER_SIZE>>>> =
    Mutex::new(RefCell::new(None));
static UART_TO_CDC_CONSUMER: Mutex<RefCell<Option<Consumer<'static, u8, FIFO_BUFFER_SIZE>>>> =
    Mutex::new(RefCell::new(None));

static mut CORE1_STACK: bsp_hal::multicore::Stack<CORE1_STACK_SIZE> =
    bsp_hal::multicore::Stack::new();

static LED_PIN: Mutex<RefCell<Option<DynLedPin>>> = Mutex::new(RefCell::new(None));
static LED_STATE: AtomicBool = AtomicBool::new(false);
static mut LED_LAST_ACTIVITY_US: u64 = 0;
pub static USB_EVENT: AtomicBool = AtomicBool::new(false);

struct Core1Data;
static CORE1_DATA: Mutex<RefCell<Option<Core1Data>>> = Mutex::new(RefCell::new(None));

fn set_led_state(state: bool) {
    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut led) = LED_PIN.borrow(cs).borrow_mut().as_mut() {
            if state {
                let _ = led.set_high().is_ok();
            } else {
                let _ = led.set_low().is_ok();
            }
            LED_STATE.store(state, Ordering::Relaxed);
        } else {
            core::panic!("LED_PIN initialization failed");
        }
    });
}

fn process_received_byte(byte: u8) {
    cortex_m::interrupt::free(|cs| {
        if let Some(ref mut producer) = UART_TO_CDC_PRODUCER.borrow(cs).borrow_mut().as_mut() {
            match producer.enqueue(byte) {
                Ok(()) => {}
                Err(_) => {
                    LED_STATE.store(true, Ordering::Relaxed);
                }
            }
        } else {
            core::panic!("USB producer not initialized");
        }
    });
}

fn core1_task() {
    loop {
        if uart_core1::is_readable() {
            let data = uart_core1::read_byte();
            process_received_byte(data);
        }

        cortex_m::interrupt::free(|cs| {
            if let Some(ref mut consumer) = CDC_TO_UART_CONSUMER.borrow(cs).borrow_mut().as_mut() {
                while let Some(b) = consumer.dequeue() {
                    while uart_core1::is_tx_full() {}
                    uart_core1::write_byte(b);
                }
            }
        });
    }
}

fn initialize_fifo_buffers() -> AppResult<()> {
    let (cdc_to_uart_producer, cdc_to_uart_consumer): (
        Producer<u8, FIFO_BUFFER_SIZE>,
        Consumer<'static, u8, FIFO_BUFFER_SIZE>,
    ) = unsafe {
        let queue_ptr = core::ptr::addr_of_mut!(CDC_TO_UART_QUEUE);
        (*queue_ptr).split()
    };

    cortex_m::interrupt::free(|cs| {
        *CDC_TO_UART_PRODUCER.borrow(cs).borrow_mut() = Some(cdc_to_uart_producer);
        *CDC_TO_UART_CONSUMER.borrow(cs).borrow_mut() = Some(cdc_to_uart_consumer);
    });

    let (uart_to_cdc_producer, uart_to_cdc_consumer): (
        Producer<u8, FIFO_BUFFER_SIZE>,
        Consumer<'static, u8, FIFO_BUFFER_SIZE>,
    ) = unsafe {
        let queue_ptr = core::ptr::addr_of_mut!(UART_TO_CDC_QUEUE);
        (*queue_ptr).split()
    };

    cortex_m::interrupt::free(|cs| {
        *UART_TO_CDC_PRODUCER.borrow(cs).borrow_mut() = Some(uart_to_cdc_producer);
        *UART_TO_CDC_CONSUMER.borrow(cs).borrow_mut() = Some(uart_to_cdc_consumer);
    });

    Ok(())
}

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().expect("Failed to take PAC peripherals");
    let core = cortex_m::Peripherals::take().expect("Failed to take Core peripherals");
    let mut sio = Sio::new(pac.SIO);

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        EXTERNAL_XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .expect("Failed to initialize clocks");

    let system_freq = clocks.system_clock.freq().to_Hz();

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    match board::init_pins_and_enable_uart(pins, pac.UART0, &mut pac.RESETS, system_freq) {
        Ok(led_local) => {
            cortex_m::interrupt::free(|cs| {
                *LED_PIN.borrow(cs).borrow_mut() = Some(led_local);
            });
        }
        Err(_e) => {
            cortex_m::asm::bkpt();
        }
    }

    initialize_fifo_buffers().expect("FIFO buffer initialization failed");

    // Initialize UART pointer for Core1 before spawning
    uart_core1::init_uart_ptr();

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    cortex_m::interrupt::free(|cs| {
        *CORE1_DATA.borrow(cs).borrow_mut() = Some(Core1Data);
    });

    #[cfg(feature = "rp2350")]
    {
        let stack_alloc = unsafe {
            core::ptr::addr_of_mut!(CORE1_STACK)
                .as_mut()
                .expect("CORE1_STACK pointer invalid")
                .take()
                .expect("Failed to take CORE1_STACK")
        };
        core1
            .spawn(stack_alloc, core1_task)
            .expect("Failed to spawn Core1 task");
    }

    #[cfg(not(feature = "rp2350"))]
    {
        let stack_slice: &'static mut [usize] = unsafe {
            let ptr = core::ptr::addr_of_mut!(CORE1_STACK) as *mut usize;
            core::slice::from_raw_parts_mut(ptr, CORE1_STACK_SIZE / core::mem::size_of::<usize>())
        };

        core1
            .spawn(stack_slice, core1_task)
            .expect("Failed to spawn Core1 task");
    }

    #[cfg(feature = "rp2040")]
    let timer = board::make_timer(pac.TIMER, &mut pac.RESETS, &clocks);
    #[cfg(feature = "rp2350")]
    let timer = board::make_timer(pac.TIMER0, &mut pac.RESETS, &clocks);

    let now = timer.get_counter().ticks();
    unsafe {
        LED_LAST_ACTIVITY_US = now;
    }
    LED_STATE.store(false, Ordering::Relaxed);
    set_led_state(false);

    #[cfg(feature = "rp2040")]
    usb_serial::init_usb(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        &mut pac.RESETS,
    );

    #[cfg(feature = "rp2350")]
    usb_serial::init_usb(pac.USB, pac.USB_DPRAM, clocks.usb_clock, &mut pac.RESETS);

    let mut delay = cortex_m::delay::Delay::new(core.SYST, system_freq);
    let mut last_usb_activity = false;

    loop {
        let mut usb_activity = usb_serial::handle_usb_serial();

        if crate::USB_EVENT.load(core::sync::atomic::Ordering::Relaxed) {
            crate::USB_EVENT.store(false, core::sync::atomic::Ordering::Relaxed);
            usb_activity = true;
            let now = timer.get_counter().ticks();
            unsafe {
                LED_LAST_ACTIVITY_US = now;
            }
            set_led_state(true);
        } else if usb_activity {
            let now = timer.get_counter().ticks();
            unsafe {
                LED_LAST_ACTIVITY_US = now;
            }
        }

        if usb_activity != last_usb_activity {
            if usb_activity {
                set_led_state(true);
            } else {
                set_led_state(false);
            }
            last_usb_activity = usb_activity;
        }

        {
            let now = timer.get_counter().ticks();
            let last = unsafe { LED_LAST_ACTIVITY_US };
            let elapsed = now.wrapping_sub(last);
            let duration = 10_000u64; // 10 ms
            if elapsed <= duration {
                set_led_state(true);
            } else {
                set_led_state(false);
            }
        }

        delay.delay_us(1000u32);
    }
}
