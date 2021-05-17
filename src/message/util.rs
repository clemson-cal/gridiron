//! Utility functions intended for use within the [`crate::message`] module.

use std::io::prelude::*;

/// Compute the log-base-two of the next power of two: 8 -> 3, 9 -> 4.
pub fn ceil_log2(x: usize) -> usize {
    let mut n = 0;
    while 1 << n < x {
        n += 1
    }
    n
}

/// Read a `usize` out of the given stream.
pub fn read_usize<R: Read>(stream: &mut R) -> usize {
    usize::from_le_bytes(read_bytes_array(stream))
}

/// If any bytes can be read immediately from a stream, then read a `usize`
/// from it and return `Some`. Otherwise return `None`
pub fn read_usize_non_blocking<R: Read>(stream: &mut R) -> Option<usize> {
    read_bytes_array_non_blocking(stream).map(usize::from_le_bytes)
}

/// Read the given number of bytes from a stream, into a `Vec<u8>`.
pub fn read_bytes_vec<R: Read>(stream: &mut R, size: usize) -> Vec<u8> {
    let mut buffer = vec![0; size];
    read_bytes_into(stream, &mut buffer);
    buffer
}

/// If any bytes can be read immediately from a stream, the read the given
/// number of bytes from it, returning `Some(Vec<u8>)`. Otherwise, return
/// `None`.
pub fn _read_bytes_vec_non_blocking<R: Read>(stream: &mut R, size: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0; size];
    read_bytes_into_non_blocking(stream, &mut buffer).map(|_| buffer)
}

/// Read the given (const) number of bytes from a stream, into an array.
pub fn read_bytes_array<R: Read, const SIZE: usize>(stream: &mut R) -> [u8; SIZE] {
    let mut buffer = [0; SIZE];
    read_bytes_into(stream, &mut buffer);
    buffer
}

/// If any bytes can be read immediately from a stream, the read the given
/// (const) number of bytes from it, returning `Some([u8; SIZE]). Otherwise,
/// return `None`.
pub fn read_bytes_array_non_blocking<R: Read, const SIZE: usize>(stream: &mut R) -> Option<[u8; SIZE]> {
    let mut buffer = [0; SIZE];
    read_bytes_into_non_blocking(stream, &mut buffer).map(|_| buffer)
}

/// Fill up the given buffer by reading bytes from a stream repeatedly until
/// the buffer is full.
pub fn read_bytes_into<R: Read>(stream: &mut R, buffer: &mut [u8]) {
    let mut cursor = 0;
    while cursor < buffer.len() {
        cursor += stream.read(&mut buffer[cursor..]).unwrap_or(0);
    }
}

/// If a message is ready to be received, then fill up the given buffer by
/// reading bytes from a stream repeatedly until the buffer is full.
/// Otherwise, return immediately.
pub fn read_bytes_into_non_blocking<R: Read>(stream: &mut R, buffer: &mut [u8]) -> Option<()> {
    let cursor = stream.read(&mut *buffer).unwrap_or(0);
    if cursor == 0 {
        None
    } else {
        read_bytes_into(stream, &mut buffer[cursor..]);
        Some(())
    }
}
