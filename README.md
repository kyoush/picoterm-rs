# RP2040/RP2350 USB-UART Bridge

A high-quality reference implementation of a USB CDC-ACM to UART bridge for Raspberry Pi Pico (RP2040) and Pico 2 (RP2350), written in Rust.

[日本語版はこちら](README-ja.md)

## Features

- **Dual-core Architecture**: Core0 handles USB, Core1 handles UART
- **Lock-free Communication**: High-performance SPSC FIFOs between cores
- **Multi-board Support**: Single codebase for both RP2040 and RP2350
- **USB CDC-ACM**: Standard serial port interface (no custom drivers needed)
- **High Throughput**: 115200 baud UART with minimal latency
- **Safe Rust**: Minimal unsafe code with clear safety documentation

## Hardware Requirements

- **Raspberry Pi Pico (RP2040)** or **Raspberry Pi Pico 2 (RP2350)**
- USB cable for power and data
- UART device connected to GPIO0 (TX) and GPIO1 (RX)

## Software Requirements

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Install Required Tools

```bash
# Add ARM targets
rustup target add thumbv6m-none-eabi      # For RP2040
rustup target add thumbv8m.main-none-eabihf  # For RP2350

# Install tools
cargo install flip-link
cargo install elf2uf2-rs --locked
```

## Building

### For Raspberry Pi Pico (RP2040)

```bash
cargo rp2040-build
# or with release optimization
cargo rp2040-build --release
```

### For Raspberry Pi Pico 2 (RP2350)

```bash
cargo rp2350-build
# or with release optimization
cargo rp2350-build --release
```

## Flashing

### Method 1: Using BOOTSEL Mode (Recommended)

1. Hold the BOOTSEL button on your Pico
2. Connect USB cable to your computer
3. Release BOOTSEL button (Pico mounts as USB drive)
4. Flash the firmware:

```bash
# For RP2040
cargo rp2040-run
# or
cargo rp2040-run --release

# For RP2350
cargo rp2350-run
# or
cargo rp2350-run --release
```

The device will automatically reboot and start running.

### Method 2: Using Debug Probe (Optional)

If you have a debug probe (e.g., Raspberry Pi Debug Probe):

1. Uncomment the probe-rs runner in `.cargo/config.toml`
2. Connect your debug probe
3. Run: `cargo run --features rp2040` (or `rp2350`)

## Usage

### Wiring

```
Pico GPIO0 (TX) ──→ Target Device RX
Pico GPIO1 (RX) ←── Target Device TX
Pico GND ────────── Target Device GND
```

### Connecting

Once flashed, the Pico appears as a USB serial port:

- **Linux**: `/dev/ttyACM0` (or similar)
- **macOS**: `/dev/tty.usbmodemXXXX`
- **Windows**: `COMX`

### Example: Using screen

```bash
# Linux/macOS
screen /dev/ttyACM0 115200

# Exit screen: Ctrl+A then K then Y
```

### Example: Using minicom

```bash
minicom -D /dev/ttyACM0 -b 115200
```

### Example: Using Python

```python
import serial

ser = serial.Serial('/dev/ttyACM0', 115200)
ser.write(b'Hello UART\n')
response = ser.read(100)
print(response)
ser.close()
```

## Project Structure

```
picoterm-rs/
├── .cargo/
│   └── config.toml        # Build configuration and aliases
├── src/
│   ├── main.rs            # Main entry point and core logic
│   ├── uart_core1.rs      # UART handling on Core1
│   ├── usb_serial.rs      # USB serial abstraction
│   └── board/             # Board-specific implementations
│       ├── mod.rs         # Board selection
│       ├── bsp.rs         # HAL re-exports
│       ├── rp2040/
│       │   ├── mod.rs     # RP2040-specific config
│       │   └── usb.rs     # RP2040 USB implementation
│       └── rp2350/
│           ├── mod.rs     # RP2350-specific config
│           └── usb.rs     # RP2350 USB implementation
├── memory.x               # Linker script
├── Cargo.toml             # Project dependencies
└── README.md              # This file
```

## Architecture

### Core Assignment

- **Core0**: USB device handling and USB CDC-ACM protocol
- **Core1**: UART communication (115200 baud, 8N1)

### Data Flow

```
PC ←─USB─→ Core0 ←─FIFO─→ Core1 ←─UART─→ External Device
```

### Inter-core Communication

Lock-free SPSC (Single Producer Single Consumer) FIFOs provide high-performance, safe data transfer:

- `CDC_TO_UART_QUEUE`: USB → UART (16KB buffer)
- `UART_TO_CDC_QUEUE`: UART → USB (16KB buffer)

## Configuration

Edit `src/main.rs` to customize:

```rust
const UART_BAUD_RATE: u32 = 115_200;  // UART baud rate
const FIFO_BUFFER_SIZE: usize = 16384; // Buffer size per direction
```

UART pins are configured in `src/board/rp2040/mod.rs` (or `rp2350/mod.rs`):

- GPIO0: UART TX
- GPIO1: UART RX
- GPIO25: LED indicator

## LED Indicator

The onboard LED indicates activity:

- **Solid**: Active USB communication
- **Blink**: Recent activity (10ms window)
- **Off**: No activity

## Troubleshooting

### Device not recognized as USB serial port

1. Check USB cable supports data (not charge-only)
2. Verify firmware flashed successfully
3. Try different USB port
4. Check device manager/dmesg for errors

### No data transmission

1. Verify UART wiring (TX ↔ RX, common ground)
2. Check baud rate matches target device
3. Confirm target device is powered
4. Test with loopback (connect GPIO0 to GPIO1)

### Build errors

```bash
# Clean and rebuild
cargo clean
cargo rp2040-build
```

### "USB already initialized" panic

This indicates a bug in initialization. Please report as an issue.

## Development

### Running Tests

```bash
# Check code with clippy
cargo clippy --target thumbv6m-none-eabi --features rp2040 --no-default-features
cargo clippy --target thumbv8m.main-none-eabihf --features rp2350 --no-default-features

# Format code
cargo fmt
```

### Adding Features

The architecture supports easy extension:

1. Add board-specific code in `src/board/<board_name>/`
2. Update `src/board/mod.rs` to include new board
3. Add feature flag in `Cargo.toml`
4. Add build alias in `.cargo/config.toml`

## License

MIT

## Contributing

Contributions are welcome! Please:

1. Follow existing code style (run `cargo fmt`)
2. Add tests where applicable
3. Update documentation
4. Ensure both RP2040 and RP2350 builds pass

## References

- [RP2040 Datasheet](https://datasheets.raspberrypi.com/rp2040/rp2040-datasheet.pdf)
- [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Raspberry Pi Pico SDK](https://github.com/raspberrypi/pico-sdk)
- [usb-device Crate](https://docs.rs/usb-device/)
- [rp2040-hal](https://docs.rs/rp2040-hal/)
- [rp235x-hal](https://docs.rs/rp235x-hal/)

## Acknowledgments

This project demonstrates safe, efficient embedded Rust patterns for the RP2040/RP2350 platform.
