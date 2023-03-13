//! This modules aggregates the parsers to retrieve hostm sockets and processes information from
//! commands output retrieved from target machines

mod csv;
pub mod directory_scanner;
pub mod linux;
mod nmap;
pub mod windows;
