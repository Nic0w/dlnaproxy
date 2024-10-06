use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {

    #[error("No SSDP method found while parsing packet.")]
    NoSSDPMethod
}
