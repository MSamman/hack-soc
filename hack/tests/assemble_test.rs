//! Golden-file tests: every `tests/fixtures/*.asm` is checked against the
//! `.hack` file beside it. Each pair runs as its own test case.

use std::{fs, path::Path};

use anyhow::{Context, anyhow};

use hack::{assembler::assemble, board::Board};

fn verify_assembler_output(asm_path: &Path, asm: String) -> datatest_stable::Result<()> {
    let expected_path = asm_path.with_extension("hack");
    let expected =
        fs::read_to_string(&expected_path).with_context(|| expected_path.display().to_string())?;
    let got = assemble(&asm, &Board::default())?.to_string();

    if got.trim_end() != expected.trim_end() {
        let (line, g, w) = first_diff(&got, &expected);
        return Err(anyhow!(
            "mismatch vs {} at line {line}:\n  got:      {g}\n  expected: {w}",
            expected_path.display()
        )
        .into());
    }
    Ok(())
}

fn first_diff(got: &str, expected: &str) -> (usize, String, String) {
    let mut g = got.lines();
    let mut e = expected.lines();
    let mut n = 0;
    loop {
        n += 1;
        match (g.next(), e.next()) {
            (Some(a), Some(b)) if a.trim_end() == b.trim_end() => continue,
            (a, b) => {
                return (
                    n,
                    a.unwrap_or("<eof>").to_string(),
                    b.unwrap_or("<eof>").to_string(),
                );
            }
        }
    }
}

datatest_stable::harness! {
    { test = verify_assembler_output, root = "tests/fixtures", pattern = r"\.asm$" },
}
