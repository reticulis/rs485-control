use std::error::Error;
use std::fmt;
use std::fmt::Formatter;

pub static CGOBJECT: &str = "Couldn't get object!";
pub static CGWINDOW: &str = "Couldn't get window!";
pub static RELAY_OPEN: [u8; 7] = [0x01, 0x03, 0x02, 0x00, 0x01, 0x79, 0x84];
pub static RELAY_CLOSE: [u8; 7] = [0x01, 0x03, 0x02, 0x00, 0x00, 0xB8, 0x44];
pub static APPLICATION_ID: &str = "com.github.reticulis.rs485-control";

pub enum TypeData {
    ASCII(Vec<u8>),
    MODBUS(Vec<u8>),
}

#[derive(Debug)]
pub struct ValidId;

impl Error for ValidId {}

impl fmt::Display for ValidId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Not found device!")
    }
}

#[derive(Debug)]
pub struct NotFoundDevices;

impl Error for NotFoundDevices {}

impl fmt::Display for NotFoundDevices {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Not found devices!")
    }
}
