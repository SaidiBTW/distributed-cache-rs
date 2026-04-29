#[derive(Debug)]
pub enum Response {
    Ok(Vec<u8>),
    NotFound,
    Err(String),
}
