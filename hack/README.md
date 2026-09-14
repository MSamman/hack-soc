Rust toolchain for assembling Hack assembly files and loading bitcode onto an FPGA over serial port.

The SoC HDL is located in `../hdl`.

## Building

The Nix dev shell at the repo root provides the Rust toolchain along with `pkg-config` and `udev`, which the `serialport` crate needs on Linux:

```sh
nix develop
```

Then build from the repo root or from this folder:

```sh
make hack                # from the repo root
cargo build --release    # from hack/
```

The binary is written to `hack/target/release/hack`. The examples below assume it is on your `PATH`; otherwise substitute `cargo run --release --`.

## Usage

```
hack <COMMAND>

Commands:
  assemble  Assemble a .asm file to .hack text or raw binary
  load      Assemble a .asm file and send it to a board over serial
  ports     List available serial ports
```

### `assemble`

```
hack assemble [OPTIONS] <ASM_PATH>
```

| Option | Default | Description |
|--------|---------|-------------|
| `-b, --board <PATH>` | built-in `nand2tetris-simulator` | Board config TOML (see `../hdl/boards/*/board.toml`) |
| `-o, --out <stdout\|file>` | `stdout` | Where to write the output |
| `--format <hack\|bin>` | `hack` | Output format, only applies with `--out file` |
| `--file <PATH>` | `<ASM_PATH>` with `.hack` or `.bin` extension | Output path, only valid with `--out file` |

```sh
# Print .hack text to the terminal
hack assemble Prog.asm

# Write Prog.hack next to Prog.asm
hack assemble Prog.asm -o file

# Write a raw binary for the Go Board
hack assemble -b ../hdl/boards/go-board/board.toml Prog.asm -o file --format bin
```

### `load`

```
hack load [OPTIONS] --port <PORT> <ASM_PATH>
```

| Option | Description |
|--------|-------------|
| `-b, --board <PATH>` | Board config TOML, as for `assemble` |
| `--port <PORT>` | Serial port name, as printed by `hack ports` |

```sh
hack ports
hack load -b ../hdl/boards/go-board/board.toml --port /dev/ttyUSB0 Prog.asm
```

## Testing

```sh
cargo test               # unit tests + golden-file tests
cargo nextest run        # same, with the nicer runner from the dev shell
cargo llvm-cov nextest   # coverage
```

Unit tests live alongside the code in each module. Golden-file tests in `tests/assemble.rs` assemble every `tests/fixtures/*.asm` against the default board and compare the result line-by-line with the `.hack` file of the same name. Each pair runs as its own test case, so one can be run by name:

```sh
cargo test --test assemble -- Pong
```

To add a case, drop an `X.asm` and its expected `X.hack` into `tests/fixtures/`.

## Code layout

| Path | Contents |
|------|----------|
| `src/main.rs` | CLI (`clap`) |
| `src/assembler.rs` | Two-pass assembler; `Bitcodes` output as `.hack` text or big-endian binary |
| `src/assembler/parser.rs` | Parses source into A-, C-, and L-instructions |
| `src/assembler/code.rs` | C-instruction encoding |
| `src/assembler/symbol.rs` | Symbol and value types |
| `src/assembler/symbol_table.rs` | Predefined and board symbols, variable allocation |
| `src/board.rs` | Board config TOML parsing and the default board |
| `src/programmer.rs` | Serial protocol: command encoding, response decoding, CRC |
| `tests/assemble.rs` | Golden-file test harness |
| `tests/fixtures/` | `.asm` / `.hack` test pairs |
