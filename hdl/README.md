SystemVerilog for a board-agnostic Hack computer (CPU, ROM, RAM), built up from nand2tetris gates. Each folder
under `boards/` wraps that core into an SoC for one board, adding memory-mapped peripherals and pin constraints.

Tool setup is covered in the [top-level README](../README.md#getting-started). Commands below run from `hdl/`.

## Structure

| Path | Contents |
|------|----------|
| `gates/` | Not, And, Or, Xor, Mux, Dmux and their 16-bit / multi-way variants |
| `adders/` | HalfAdder, FullAdder, 16-bit Adder, Incrementer |
| `alu/` | ALU |
| `memory/` | DFF, Register, Counter, and the nand2tetris RAM8 → RAM16k |
| `computer/` | CPU, ROM, RAM: the board-agnostic Hack computer |
| `uart/` | UART receiver and transmitter (115200 baud, 8N1) |
| `boards/` | One folder per board: synthesis top, memory map, peripherals, pin constraints |
| `scripts/` | `pack_bitstream.sh` (icepack) and `flash-bitstream.sh` (iceprog), called by the Makefile. `synthesize.sh` and `pnr.sh` are not used by the Makefile. |

## Core architecture

The core is three modules in `computer/`:

| Module | Role |
|--------|------|
| `CPU` | Hack CPU: A and D registers, ALU, program counter. Drives `addressM`, `outM`, `writeM` and `pc`. |
| `ROM #(WORDS)` | Instruction memory, filled from a file by `$readmemb` at elaboration (see [Program ROM](#program-rom)) |
| `RAM #(WORDS)` | Data memory |

ROM and RAM register their reads on the falling clock edge and RAM writes on the rising edge. This lets both infer
iCE40 block RAM while the CPU still sees read data within the same cycle. The cost is that ROM → decode → ALU →
register setup must fit in half a clock period, which is why the Go Board runs its CPU at 12.5 MHz.

There is no shared top module. Each board's `top.sv` instantiates `CPU`, `ROM` and `RAM`, sizes the memories from its
`board_map.svh`, and decodes `addressM` between RAM and its peripherals.

## Program ROM

The program is baked into the bitstream: `ROM.sv` loads it with `$readmemb`, so the file must be text with one
16-digit binary word per line. That is the `hack` format from `hack assemble`, **not** `--format bin`, which writes
raw bytes.

Select the program with `PROGRAM=`, a path relative to `hdl/`. The Makefile passes it to yosys as the `PROGRAM_BIN`
define. The file is a prerequisite of the synthesized netlist, so changing it triggers a new synthesis run.

```sh
../hack/target/release/hack assemble -b boards/go-board/board.toml boards/go-board/test.asm -o file
make bitstream BOARD=go-board PROGRAM=boards/go-board/test.hack
```

`PROGRAM` defaults to `program.hex`. `ROM.sv` falls back to `program.bin` only when nothing defines `PROGRAM_BIN`.
`ROM_tb` defines its own, pointing at `computer/rom_test.bin`.

## Building for an FPGA

Board targets need `BOARD=`, and take their device, package and timing target from `boards/<BOARD>/board.mk`
(defaults: `hx1k`, `tq144`). Each target builds the ones before it. Run them from `hdl/`, or with `make -C hdl …`
from the repo root.

```sh
make boards                                                     # list available boards
make synth      BOARD=go-board PROGRAM=boards/go-board/test.hack  # yosys    → build/go-board.json
make pnr        BOARD=go-board PROGRAM=boards/go-board/test.hack  # nextpnr  → build/go-board.asc
make bitstream  BOARD=go-board PROGRAM=boards/go-board/test.hack  # icepack  → build/go-board.bin
make flash      BOARD=go-board PROGRAM=boards/go-board/test.hack  # iceprog
make lint-board BOARD=go-board                                  # Verilator lint of one board
make attach     BOARD=go-board                                  # WSL: attach the board's USB via usbipd
```

Pass the same `PROGRAM=` to every step, otherwise a rebuild falls back to `program.hex`. Outputs go to `build/`,
which is gitignored.

nextpnr prints `unmatched constraint` warnings for the UART, VGA and PMOD pins. They are expected: the Go Board
`.pcf` lists every pin on the board, and `top` only uses some of them.

## Boards

Every board lives in `boards/<name>/` and shows up in `make boards` automatically.

| File | Purpose |
|------|---------|
| `top.sv` | Synthesis top. Must be named `top`: the `.pcf` constrains its ports by name. |
| `board_map.svh` | Memory map for the HDL: `ROM_WORDS`, `RAM_WORDS`, peripheral addresses |
| `board.toml` | The same memory map for the assembler (`hack -b`) |
| `*.pcf` | Pin constraints. Boards without one can be linted but not placed. |
| `board.mk` | `DEVICE`, `PACKAGE`, `FREQ` (nextpnr timing target in MHz), `USB_BUSID` (WSL `attach`) |
| other `*.sv` | Board peripherals. Every `.sv` in the folder is synthesized and linted with `top`. |

`board_map.svh` and `board.toml` describe the same memory map with different names (`LED_ADDR` vs `LED`). Nothing
checks that they agree, so change both together.

### Go Board

Nandland Go Board: Lattice iCE40 HX1K-VQ100 with a 25 MHz oscillator. The CPU runs at 12.5 MHz (the oscillator
divided by two) and is held in reset for 8 cycles after power-up. There is no reset button yet.

| Memory | Size |
|--------|------|
| ROM | `0xC00` words (3K, 12 block RAMs) |
| RAM | `0x400` words (1K, 4 block RAMs) |

| Address | Symbol | Description |
|---------|--------|-------------|
| `0x0000`–`0x03FF` | | RAM |
| `0x4000` | `LED` | Read/write. Bits 3:0 drive LED4–LED1. |
| `0x4001` | `SEG` | Read/write. Bits 7:0 shown as two hex digits, high nibble on the left display. |
| `0x4002` | `BTN` | Reserved, not wired yet |
| `0x4003`–`0x4005` | `UART_TX`, `UART_RX`, `UART_ST` | Reserved, not wired yet |

Address bit 14 selects I/O over RAM. RAM decodes only the low 10 bits and I/O the low 3 bits, so both regions repeat
(`0x0400` is `0x0000`, `0x4008` is `LED`).

### nand2tetris-simulator

The standard Hack memory map: 32K-word ROM, 16K-word RAM, and `SCREEN` (`0x4000`) and `KBD` (`0x6000`) as symbols.
The assembler uses this map when `hack` is run without `-b`. `top.sv` wires only CPU, ROM and RAM, with no screen or
keyboard hardware; RAM decodes the low 14 bits, so writes to `SCREEN` or `KBD` land in RAM. It has no `.pcf`, so it
is covered by `make lint` but cannot be placed.

### Adding a board

1. Create `boards/<name>/` with `top.sv` (module `top`), `board_map.svh` and `board.toml`.
2. Add a `.pcf` whose pin names match `top`'s ports, and a `board.mk` if the part is not an HX1K in a TQ144 package.
3. Check it with `make lint-board BOARD=<name>`, then build with `make bitstream BOARD=<name> PROGRAM=…`.

## Testing

```sh
make                # run every testbench (default target)
make TEST=CPU_tb    # run a single testbench
make lint           # lint the core and every board that has HDL
make clean          # remove build/
```

The same targets work from the repo root.

- A testbench is `X_tb.sv` next to `X.sv`, and is picked up automatically. Its module name must match the file name.
- Every testbench is compiled with all of `gates/`, `adders/`, `alu/`, `memory/` and `uart/`. Testbenches in
  `computer/` are also linked with the `computer/` sources.
- Compiled testbenches are cached in `build/`, so re-running only recompiles what changed. The simulation itself
  re-runs every time.
- Each testbench writes a waveform to `/tmp/<name>.vcd`, e.g. `gtkwave /tmp/CPU_tb.vcd`.
