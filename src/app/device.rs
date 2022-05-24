use crate::app::values::ValidId;
use crc::{Crc, CRC_16_MODBUS};
use serialport::{DataBits, Parity, SerialPort, SerialPortInfo, StopBits};
use std::error::Error;
use std::io::{Read, Write};
use std::time::Duration;

pub fn rs485_write(port: &mut Box<dyn SerialPort>, buf: &[u8]) -> Result<(), Box<dyn Error>> {
    port.write_all(buf)?;
    Ok(())
}

pub fn rs485_read(port: &mut Box<dyn SerialPort>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut read_buf = Vec::new();
    match port.read_to_end(&mut read_buf) {
        Ok(_) => Ok(read_buf),
        Err(e) => {
            if read_buf.is_empty() {
                Err(Box::new(e))
            } else {
                Ok(read_buf)
            }
        }
    }
}

pub fn control_command(id: u8, command: u8) -> Vec<u8> {
    let mut control = vec![0x01, 0x06, 0x00, id + 1, command, 0x00];
    checksum(&mut control);

    control
}

pub fn read_status_command(id: u8) -> Vec<u8> {
    let mut read_status = vec![0x01, 0x03, 0x00, id + 1, 0x00, 0x01];
    checksum(&mut read_status);

    read_status
}

pub fn checksum(vec: &mut Vec<u8>) {
    let checksum = Crc::<u16>::new(&CRC_16_MODBUS).checksum(&vec);
    vec.push(((checksum << 8) >> 8) as u8);
    vec.push((checksum >> 8) as u8);
}

pub fn set_port(
    ports: &Vec<SerialPortInfo>,
    id: u8,
) -> Result<Box<dyn SerialPort>, Box<dyn Error>> {
    let id_ports = match ports.get(id as usize) {
        Some(v) => v,
        None => return Err(Box::new(ValidId)),
    };
    Ok(serialport::new(&*id_ports.port_name, 9600)
        .timeout(Duration::from_millis(100))
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .open()?)
}
