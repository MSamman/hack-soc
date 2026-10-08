use rand::prelude::*;
use rand::{SeedableRng, rngs::{self, StdRng}};
use rand::distr::{Alphanumeric, SampleString};


const DESTS: &[&str] = &["", "M", "D", "MD", "DM", "A", "AM", "AD", "AMD", "MDA"];
const COMPS: &[&str] = &[
    "0", "1", "-1", "D", "A", "M", "!D", "!A", "!M", "-D", "-A", "-M",
    "D+1", "A+1", "M+1", "D-1", "A-1", "M-1", "D+A", "D+M", "D-A", "D-M",
    "A-D", "M-D", "D&A", "D&M", "D|A", "D|M",
];
const JUMPS: &[&str] = &["", "JGT", "JEQ", "JGE", "JLT", "JNE", "JLE", "JMP"];
const SYMBOLS: &[&str] = &["R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7","R8", "R9",
    "R10", "R11", "R12", "R13", "R14", "R15", "SP", "LCL","ARG", "THIS", "THAT"];

pub fn generate(lines: usize, seed: u64) -> String {
  let mut asm = String::with_capacity(lines);

  let mut rng = rngs::StdRng::seed_from_u64(seed);

  for _ in 0..lines {
    match rng.random_range(0..100) {
      0..40 => write_a(&mut rng, &mut asm, lines),
      40..95 => write_c(&mut rng, &mut asm),
      95..99 => write_label(&mut rng, &mut asm),
      _ => write_comment(&mut rng, &mut asm),
    }
  }

  asm
}

fn write_a(rng: &mut StdRng, asm: &mut String, lines: usize) {
  asm.push('@');
  match rng.random_range(0..100) {
    0..80 => {
      asm.push_str(SYMBOLS.choose(rng).unwrap());
    }
    _ => {
      asm.push_str(& rng.random_range(0..lines).to_string());
    }
  }
  asm.push('\n');
}

fn write_c(rng: &mut StdRng, asm: &mut String) {
  match DESTS.choose(rng).unwrap() {
    d if *d != "" => {
      asm.push_str(d);
      asm.push('=');
    },
    _ => {},
  }
  asm.push_str(COMPS.choose(rng).unwrap());
  match JUMPS.choose(rng).unwrap() {
    j if *j != "" => {
      asm.push(';');
      asm.push_str(j);
    },
    _ => {},
  }

  asm.push('\n');
}

fn write_label(rng: &mut StdRng, asm: &mut String) {
  asm.push_str("(L");
  asm.push_str(Alphanumeric.sample_string(rng, 16).as_str());
  asm.push_str(")\n");

}

fn write_comment(rng: &mut StdRng, asm: &mut String) {
  asm.push_str("// comment\n".repeat(rng.random_range(0..20)).as_str())
}

