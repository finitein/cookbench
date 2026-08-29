//! Read-only remote source primitives shared by desktop transports.
//!
//! This module deliberately models hosts and polling only. Transport code lives
//! in the desktop shell so the core never opens a network connection or starts
//! a remote process.

pub mod host;

pub use host::{HostValidationError, PollInterval, RemoteHost, RemoteSessionIdentity, SessionRoot};
