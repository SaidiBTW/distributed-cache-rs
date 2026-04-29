#[repr(u8)]
pub enum Command {
    Get = 1,
    Set = 2,
    Del = 3,
}

impl TryFrom<u8> for Command {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, u8> {
        match value {
            1 => Ok(Command::Get),
            2 => Ok(Command::Set),
            3 => Ok(Command::Del),
            other => Err(other),
        }
    }
}
