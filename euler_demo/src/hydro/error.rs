#![allow(unused)]
use std::fmt;
use std::error;




#[derive(Debug)]


/**
 * Error to represent invalid hydrodynamics data or primitive variable recovery.
 */
pub enum Error {
    NegativeGasPressure(f64),
    NegativeMassDensity(f64),
}


impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        use Error::*;

        match self {
            NegativeGasPressure(p) => writeln!(fmt, "negative gas pressure: {}", p),
            NegativeMassDensity(d) => writeln!(fmt, "negative mass density: {}", d),
        }
    }
}


impl error::Error for Error {}
