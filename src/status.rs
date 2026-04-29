#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum Status {
    Ok = 0x00,
    NotFound = 0x01,
    Err = 0xFF,
}

impl TryFrom<u8> for Status {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Status::Ok),
            0x01 => Ok(Status::NotFound),
            0xFF => Ok(Status::Err),
            other => Err(other),
        }
    }
}
