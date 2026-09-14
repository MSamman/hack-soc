# Hack SoC

An implementation of the [nand2tetris](https://www.nand2tetris.org/) Hack computer in board-agnostic SystemVerilog,
extended into an SoC through board-specific peripherals, plus a Rust toolchain for assembling Hack programs and
loading them onto an FPGA over UART. The current hardware target is the Nandland Go Board (Lattice iCE40 HX1K).

## Repository layout

| Path | Contents |
|------|----------|
| [`hdl/`](hdl/README.md) | SystemVerilog: Hack CPU, UART, board-specific tops and peripherals |
| [`hack/`](hack/README.md) | Rust CLI: Hack assembler and UART programmer |
| `flake.nix` | Nix dev shell for both halves |
| `Makefile` | Top-level entry point; delegates to `hdl/Makefile` and Cargo |

## Getting started

The Nix dev shell provides everything both halves need:

```sh
nix develop
```

Without Nix, install:

- **HDL simulation:** [Icarus Verilog](https://steveicarus.github.io/iverilog/), Verilator (lint), GTKWave (waveforms)
- **FPGA:** yosys, nextpnr (iCE40), icestorm
- **hack:** a Rust toolchain (edition 2024), plus `pkg-config` and `libudev` for the `serialport` crate on Linux

Then build the CLI:

```sh
make hack           # cargo build --release; binary at hack/target/release/hack
```

Run `nix develop` from inside the repository. The dev shell derives build paths from the directory it is entered in,
and a path containing spaces (e.g. a VS Code install directory) breaks linking with `ld: cannot find ...`.

### WSL

USB devices must be attached to WSL with [usbipd-win](https://github.com/dorssel/usbipd-win) before flashing or
using the serial port. Set the board's `USB_BUSID` in `hdl/boards/<board>/board.mk`, then:

```sh
make -C hdl attach BOARD=go-board
```

## Testing

```sh
make                # run all HDL testbenches (default target)
make TEST=CPU_tb    # run a single testbench
make lint           # lint the core and every board top with Verilator
make clean          # remove build artifacts

cd hack && cargo test   # assembler and programmer tests
```

Compiled testbenches are cached in `hdl/build/`, so re-running only recompiles what changed.
The simulation itself re-runs every time. See [`hack/README.md`](hack/README.md#testing) for the Rust test layout.

## Serial protocol

`hack load` talks to the board over UART at **115200 baud**. The host side lives in
[`hack/src/programmer.rs`](hack/src/programmer.rs); the FPGA-side command decoder is not implemented yet, and only
`load` is currently exposed by the CLI.

All multi-byte fields are big-endian `u16`. Every command starts with a single ASCII opcode byte.

### Commands (host → board)

| Command | Opcode | Payload |
|---------|--------|---------|
| Ping | `'P'` (`0x50`) | none |
| Hold | `'H'` (`0x48`) | none |
| Load | `'L'` (`0x4C`) | `len`, `word[len]`, `crc` |
| Get | `'G'` (`0x47`) | `address`, `len` |
| Run | `'R'` (`0x52`) | none |
| Step | `'S'` (`0x53`) | `len` |

### Responses (board → host)

Every response starts with a status byte: `0xAA` (ACK) or `0x55` (NACK). A NACK ends the response; after an ACK
the payload depends on the command:

| Command | Payload after ACK |
|---------|-------------------|
| Ping, Hold, Run | none |
| Load | `crc` |
| Get | `len`, `word[len]`, `crc` |
| Step | `pc`, `a`, `d`, `crc` |

### CRC

CRC-16/CCITT-FALSE (`CRC_16_IBM_3740` in the `crc` crate): polynomial `0x1021`, init `0xFFFF`, no reflection,
no final XOR. It is computed over the data words only, high byte first. For Load and Get this excludes the
opcode and `len`; for Step it covers `pc`, `a` and `d`.

The receiver of a payload checks its CRC. On Load, the board checks the host's CRC, replies NACK on a mismatch, and
otherwise echoes back the CRC it computed so the host can cross-check the two implementations. On Get and Step,
the host checks the board's CRC. The host does not check any response CRC yet.
