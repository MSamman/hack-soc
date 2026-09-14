`include "board_map.svh"

module top(
  input   logic       clk,
  output  logic[3:0]  ledsOut,
  output  logic[6:0]  segment1Out,
  output  logic[6:0]  segment2Out
);
  wire  logic[15:0] instruction;
  wire  logic[15:0] pc;
  wire  logic[15:0] inM, outC, outP, outR, addressM;

  wire        ioSelect = addressM[14];
  wire        write;
  wire        writeP = write & ioSelect;
  wire        writeM = write & ~ioSelect;

  assign inM = ioSelect ? outP : outR;
  
  // CPU at 12.5 MHz: the negedge-read BRAM leaves only half a period for
  // ROM -> decode -> ALU -> setup, which does not close at 25 MHz.
  logic clkCpu = 1'b0;
  always_ff @(posedge clk) clkCpu <= ~clkCpu;

  ROM #(.WORDS(`ROM_WORDS)) computerROM(
    .clk(clkCpu),
    .address(pc[$clog2(`ROM_WORDS)-1:0]),
    .instruction(instruction)
  );

  // TODO tie Reset to actual input (button?). For now hold reset for 8 cycles then release
  logic [3:0] por = 4'd0;
  wire  reset = ~por[3];
  always_ff @(posedge clkCpu) if (~por[3]) por <= por + 1'b1;
  CPU computerCPU(
    .clk(clkCpu),
    .inM(inM),
    .instruction(instruction),
    .reset(reset),
    .writeM(write),
    .outM(outC),
    .addressM(addressM),
    .pc(pc)
  );

  RAM #(.WORDS(`RAM_WORDS)) computeRAM(
    .clk(clkCpu),
    .address(addressM[$clog2(`RAM_WORDS)-1:0]),
    .in(outC),
    .load(writeM),
    .out(outR)
  );

  Peripherals peripherals(
    .clk(clkCpu),
    .address(addressM[`IO_SEL_BITS-1:0]),
    .load(writeP),
    .in(outC),
    .out(outP),
    .ledsOut(ledsOut),
    .segment1Out(segment1Out),
    .segment2Out(segment2Out)
  );
endmodule