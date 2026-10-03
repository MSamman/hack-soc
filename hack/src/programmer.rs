use std::io::{Read, Write};

use anyhow::{Result, bail};
use crc::CRC_16_IBM_3740;
use serialport;

const BAUD_RATE: u32 = 115200;

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum OpCode {
    Ping = 'P' as u8,
    Hold = 'H' as u8,
    Load = 'L' as u8,
    Get = 'G' as u8,
    Run = 'R' as u8,
    Step = 'S' as u8,
}

#[derive(Debug)]
pub enum SerialCommand<'a> {
    Ping,
    Hold,
    Load { words: &'a [u16] },
    Get { address: u16, len: u16 },
    Run,
    Step { len: u16 },
}

impl SerialCommand<'_> {
    const fn op_code(&self) -> OpCode {
        match self {
            Self::Ping => OpCode::Ping,
            Self::Hold => OpCode::Hold,
            Self::Load { .. } => OpCode::Load,
            Self::Get { .. } => OpCode::Get,
            Self::Run => OpCode::Run,
            Self::Step { .. } => OpCode::Step,
        }
    }

    fn write_u16<W: Write>(writer: &mut W, value: u16) -> Result<()> {
        writer.write_all(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<Option<u16>> {
        let mut crc = None;

        writer.write_all(&[self.op_code() as u8])?;
        match *self {
            Self::Ping {} | Self::Hold {} | Self::Run {} => {}
            Self::Load { words } => {
                if words.len() > u16::MAX as usize {
                    bail!("Can't write more than u16::MAX words")
                }

                let alg: crc::Crc<u16> = crc::Crc::<u16>::new(&CRC_16_IBM_3740);
                let mut digest = alg.digest();
                Self::write_u16(writer, words.len() as u16)?;
                for &w in words {
                    Self::write_u16(writer, w)?;
                    digest.update(&w.to_be_bytes());
                }
                let c = digest.finalize();
                Self::write_u16(writer, c)?;
                crc = Some(c);
            }
            Self::Get { address, len } => {
                Self::write_u16(writer, address)?;
                Self::write_u16(writer, len)?;
            }
            Self::Step { len } => {
                Self::write_u16(writer, len)?;
            }
        }

        writer.flush()?;
        Ok(crc)
    }
}

#[derive(PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    Ack = 0xAA,
    Nack = 0x55,
}

impl TryFrom<u8> for Status {
    type Error = anyhow::Error;
    fn try_from(b: u8) -> Result<Self> {
        match b {
            0xAA => Ok(Status::Ack),
            0x55 => Ok(Status::Nack),
            other => bail!("invalid status byte recieved: {other:#04x}"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum SerialResponse {
    Empty,
    Load { crc: u16 },
    Get { len: u16, words: Vec<u16>, crc: u16 },
    Step { pc: u16, a: u16, d: u16, crc: u16 },
}

fn read_u16<R: Read>(reader: &mut R, buf: &mut [u8; 2]) -> Result<u16> {
    reader.read_exact(buf)?;
    Ok(u16::from_be_bytes(*buf))
}

impl SerialResponse {
    pub fn read_from<R: Read>(reader: &mut R, op_code: OpCode) -> Result<Self> {
        let mut byte_buf = [0u8; 1];

        reader.read_exact(&mut byte_buf)?;
        let status = Status::try_from(byte_buf[0])?;
        if status == Status::Nack {
            bail!("{op_code:?} command received a NACK response")
        }

        let mut word_bytes_buf = [0u8; 2];
        Ok(match op_code {
            OpCode::Ping | OpCode::Hold | OpCode::Run => Self::Empty {},
            OpCode::Load => Self::Load {
                crc: read_u16(reader, &mut word_bytes_buf).expect("Error reading crc."),
            },
            OpCode::Get => {
                let len = read_u16(reader, &mut word_bytes_buf).expect("Error reading len.");

                let mut words = Vec::<u16>::with_capacity(len as usize);
                for _ in 0..len {
                    words.push(read_u16(reader, &mut word_bytes_buf).expect("Error reading word."));
                }

                let crc = read_u16(reader, &mut word_bytes_buf).expect("Error reading crc.");

                Self::Get { len, words, crc }
            }
            OpCode::Step => Self::Step {
                pc: read_u16(reader, &mut word_bytes_buf).expect("Error reading PC."),
                a: read_u16(reader, &mut word_bytes_buf).expect("Error reading A."),
                d: read_u16(reader, &mut word_bytes_buf).expect("Error reading D."),
                crc: read_u16(reader, &mut word_bytes_buf).expect("Error reading crc."),
            },
        })
    }
}

pub fn print_ports() {
    match serialport::available_ports() {
        Ok(ports) => println!("{:?}", ports),
        Err(e) => println!("No ports found: {e}"),
    }
}

pub fn process_command(port_name: &str, command: SerialCommand) -> Result<SerialResponse> {
    let port_info = serialport::available_ports()
        .expect("No ports found")
        .into_iter()
        .find(|p: &serialport::SerialPortInfo| p.port_name == port_name)
        .expect(format!("{port_name} was not found").as_str());

    let mut port = serialport::new(port_info.port_name.as_str(), BAUD_RATE)
        .open()
        .expect(format!("Unable to open port {port_name}").as_str());

    /*
        Todo: Check CRCs when applicable
    */
    let _command_crc = command.write_to(&mut port)?;

    SerialResponse::read_from(&mut port, command.op_code())
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use crate::programmer::{OpCode, SerialCommand, SerialResponse};

    use anyhow::Result;

    #[test]
    fn test_op_codes() {
        let tests = [
            (SerialCommand::Ping, OpCode::Ping),
            (SerialCommand::Hold, OpCode::Hold),
            (SerialCommand::Load { words: &[] }, OpCode::Load),
            (SerialCommand::Get { len: 0, address: 0 }, OpCode::Get),
            (SerialCommand::Run, OpCode::Run),
            (SerialCommand::Step { len: 0 }, OpCode::Step),
        ];

        for (command, expected_code) in tests {
            assert_eq!(command.op_code(), expected_code)
        }
    }
    #[test]
    fn test_command_write_to() -> Result<()> {
        let tests = [
            (SerialCommand::Ping, vec![b'P']),
            (SerialCommand::Hold, vec![b'H']),
            (
                SerialCommand::Load {
                    words: &[1 as u16, 2, 3],
                },
                vec![
                    b'L', 0_u8, 3_u8, 0_u8, 1_u8, 0_u8, 2_u8, 0_u8, 3_u8, 250_u8, 66_u8,
                ],
            ),
            (
                SerialCommand::Get {
                    address: 0xFF00,
                    len: 0xFF,
                },
                vec![b'G', 0xFF_u8, 0_u8, 0_u8, 0xFF_u8],
            ),
            (SerialCommand::Run, vec![b'R']),
            (
                SerialCommand::Step { len: 0x0FF0 },
                vec![b'S', 0x0F_u8, 0xF0_u8],
            ),
        ];

        for (command, expected_out) in tests {
            let mut buf = Vec::new();
            command.write_to(&mut buf)?;
            assert_eq!(buf, expected_out, "{command:?}");
        }

        Ok(())
    }

    #[test]
    fn test_response_read_from() -> Result<()> {
        let tests = [
            (vec![0xAA as u8], OpCode::Ping, SerialResponse::Empty),
            (vec![0xAA as u8], OpCode::Hold, SerialResponse::Empty),
            (vec![0xAA as u8], OpCode::Run, SerialResponse::Empty),
            (
                vec![
                    0xAA, 0_u8, 3_u8, 0_u8, 1_u8, 0_u8, 2_u8, 0_u8, 3_u8, 250_u8, 66_u8,
                ],
                OpCode::Get,
                SerialResponse::Get {
                    len: 3,
                    words: vec![1 as u16, 2, 3],
                    crc: 0xFA42,
                },
            ),
            (
                vec![0xAA, 250_u8, 66_u8],
                OpCode::Load,
                SerialResponse::Load { crc: 0xFA42 },
            ),
            (
                vec![0xAA, 0_u8, 1_u8, 0_u8, 2_u8, 0_u8, 3_u8, 250_u8, 66_u8],
                OpCode::Step,
                SerialResponse::Step {
                    pc: 1,
                    a: 2,
                    d: 3,
                    crc: 0xFA42,
                },
            ),
        ];

        for (buf, op_code, expected_response) in tests {
            let mut reader = buf.as_slice();
            let response = SerialResponse::read_from(&mut reader, op_code)?;
            assert_eq!(response, expected_response, "{response:?}");
        }

        Ok(())
    }

    #[test]
    fn test_nack_response_error() {
        let nack_byte = [0x55 as u8];
        assert!(SerialResponse::read_from(&mut nack_byte.as_slice(), OpCode::Ping).is_err());
    }
}
