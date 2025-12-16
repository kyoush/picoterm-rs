use crate::board::hal as bsp_hal;
use bsp_hal::clocks::UsbClock;
use bsp_hal::pac::RESETS;
use bsp_hal::pac::{USBCTRL_DPRAM, USBCTRL_REGS};
use bsp_hal::usb::UsbBus as HalUsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

use core::cell::RefCell;
use core::mem::MaybeUninit;
use cortex_m::interrupt::Mutex;

/// Static USB device storage (initialized once during init_usb)
static mut USB_DEVICE: MaybeUninit<UsbDevice<'static, HalUsbBus>> = MaybeUninit::uninit();
/// Static USB bus allocator storage (initialized once during init_usb)
static mut USB_BUS: MaybeUninit<UsbBusAllocator<HalUsbBus>> = MaybeUninit::uninit();
/// Static USB serial port storage (initialized once during init_usb)
static mut USB_SERIAL: MaybeUninit<SerialPort<'static, HalUsbBus>> = MaybeUninit::uninit();

/// Initialization flag to ensure single initialization
static USB_INITIALIZED: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

/// Initialize USB device and CDC serial port for RP2040
///
/// # Safety
/// Must be called exactly once before handle_usb_serial()
pub fn init_usb(
    usbctrl_regs: USBCTRL_REGS,
    usbctrl_dpram: USBCTRL_DPRAM,
    usb_clock: UsbClock,
    resets: &mut RESETS,
) {
    cortex_m::interrupt::free(|cs| {
        if *USB_INITIALIZED.borrow(cs).borrow() {
            panic!("USB already initialized");
        }
        *USB_INITIALIZED.borrow(cs).borrow_mut() = true;
    });

    unsafe {
        let bus_ptr = core::ptr::addr_of_mut!(USB_BUS);
        (*bus_ptr)
            .as_mut_ptr()
            .write(UsbBusAllocator::new(HalUsbBus::new(
                usbctrl_regs,
                usbctrl_dpram,
                usb_clock,
                true,
                resets,
            )));

        let usb_bus: &'static UsbBusAllocator<HalUsbBus> = &*(*bus_ptr).as_ptr();

        let serial_ptr = core::ptr::addr_of_mut!(USB_SERIAL);
        (*serial_ptr).as_mut_ptr().write(SerialPort::new(usb_bus));

        let dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .strings(&[StringDescriptors::new(LangID::EN)
                .manufacturer("Example")
                .product("rp-serial")
                .serial_number("000")])
            .unwrap()
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        let dev_ptr = core::ptr::addr_of_mut!(USB_DEVICE);
        (*dev_ptr).as_mut_ptr().write(dev);
    }
}

/// Handle USB serial communication
///
/// Polls USB device and transfers data between USB CDC and UART FIFOs.
/// Returns true if there was USB activity or data was transmitted.
pub fn handle_usb_serial() -> bool {
    unsafe {
        let dev_ptr = core::ptr::addr_of_mut!(USB_DEVICE);
        let dev = &mut *(*dev_ptr).as_mut_ptr();

        let serial_ptr = core::ptr::addr_of_mut!(USB_SERIAL);
        let serial = &mut *(*serial_ptr).as_mut_ptr();

        let has_usb_event = dev.poll(&mut [serial]);

        // Read from USB CDC (PC -> device -> UART)
        if has_usb_event {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    // Forward received data to UART via FIFO
                    cortex_m::interrupt::free(|cs| {
                        if let Some(ref mut producer) =
                            crate::CDC_TO_UART_PRODUCER.borrow(cs).borrow_mut().as_mut()
                        {
                            for &byte in &buf[..count] {
                                let _ = producer.enqueue(byte);
                            }
                        }
                    });
                }
                _ => {}
            }
        }

        // Forward UART data to USB CDC (always check, not just when has_usb_event)
        let mut data_sent = false;
        cortex_m::interrupt::free(|cs| {
            if let Some(ref mut consumer) =
                crate::UART_TO_CDC_CONSUMER.borrow(cs).borrow_mut().as_mut()
            {
                let mut tx_buf = [0u8; 64];
                let mut count = 0;
                while count < tx_buf.len() {
                    if let Some(byte) = consumer.dequeue() {
                        tx_buf[count] = byte;
                        count += 1;
                    } else {
                        break;
                    }
                }
                if count > 0 && serial.write(&tx_buf[..count]).is_ok() {
                    data_sent = true;
                }
            }
        });

        has_usb_event || data_sent
    }
}
