use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use hack::{assembler::assemble, board::Board};
mod generator;

fn criterion_benchmark(c: &mut Criterion) {
  let seed = 0;
  let lines = 0x7FFF;
  let asm = generator::generate(lines, seed);
  
  let board = &Board::default();
  let res = assemble(&asm, &board);
  assert!(res.is_ok(), "error assembling {}", res.err().unwrap());

  let mut group = c.benchmark_group("assembler benchmark");
  group.throughput(Throughput::Elements(lines as u64));
  group.bench_function(
    format!("seed {} with {} lines", seed, lines).as_str(),
    |b| b.iter(|| assemble(black_box(asm.as_str()),  &board))
  );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
