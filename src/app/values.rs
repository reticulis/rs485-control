pub static NFOUND: &str = "Not found devices!";
pub static CGOBJECT: &str = "Couldn't get object!";
pub static CGWINDOW: &str = "Couldn't get window!";
pub static RELAY_OPEN: [u8; 7] = [0x01, 0x03, 0x02, 0x00, 0x01, 0x79, 0x84];
pub static RELAY_CLOSE: [u8; 7] = [0x01, 0x03, 0x02, 0x00, 0x00, 0xB8, 0x44];
pub static APPLICATION_ID: &str = "com.github.reticulis.rs485-control";

pub enum TypeData {
    ASCII,
    MODBUS,
}
