{
  description = "Hack computer: SystemVerilog for ICE40 (Go board) + hack toolchain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        # Shared by both shells, so CI (`nix develop .#hdl`) runs the same
        # simulator, linter and iCE40 flow versions as the default shell.
        hdlTools = with pkgs; [
          # ICE40 FPGA toolchain
          yosys              # Synthesis tool
          nextpnr            # Place and route for ICE40 (includes ICE40 support)
          icestorm           # Bitstream generation tools (icepack, iceprog, etc.)

          # SystemVerilog/Verilog tools
          iverilog           # Icarus Verilog simulator
          verilator          # Verilog/SystemVerilog simulator and linter
        ];
      in
      {
        # Lean shell for CI: the HDL tools only, without GTKWave or Rust.
        devShells.hdl = pkgs.mkShell {
          packages = hdlTools;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config         # so the serialport crate can locate libudev
          ];

          buildInputs = hdlTools ++ (with pkgs; [
            # Rust toolchain for hack (assembler + UART programmer)
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            cargo-nextest     # nicer test runner: cargo nextest run
            cargo-llvm-cov    # coverage: cargo llvm-cov nextest
            perf              # profiler: perf record -g / perf report

            # Native dependency of the serialport crate (UART programmer)
            udev

            # Development tools
            gtkwave            # Waveform viewer
            python3            # Often needed for scripts
            python3Packages.pip

            # Utilities
            which
            git
          ]);

          # Lets rust-analyzer resolve std sources
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

          # cargo-llvm-cov normally wants rustup's llvm-tools-preview component.
          # Point it at the exact LLVM that rustc was built with instead, so the
          # .profraw format always matches.
          LLVM_COV = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-cov";
          LLVM_PROFDATA = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-profdata";

          shellHook = ''
            echo "Hack Computer Development Environment"
            echo "===================================="
            echo "HDL:     iverilog, verilator, yosys, nextpnr, icepack, iceprog, gtkwave"
            echo "hack:    cargo, rustc, clippy, rustfmt, rust-analyzer, cargo-nextest, cargo-llvm-cov, perf"
            echo ""
            echo "  make test              # run all HDL testbenches"
            echo "  make TEST=CPU_tb       # run a single testbench"
            echo "  make lint              # lint with Verilator"
            echo "  make hack              # build the Rust CLI"
            echo ""
          '';
        };
      }
    );
}
