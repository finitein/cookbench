//! Read-only boundaries for integrating external coding-agent harnesses.
//!
//! Adapters observe native session data and offer user-mediated resume hints;
//! they never launch, host, or control an agent process.

mod adapter;
mod capabilities;
pub mod claude;
pub mod codex;
mod fixture;
pub mod io;
mod registry;

pub use adapter::{
    AdapterError, AdapterEvent, EventSink, HarnessAdapter, HostSource, NativeSession, ResumeAction,
    SessionLocator, SessionLocatorKind, WatchHandle,
};
pub use capabilities::AdapterCapabilities;
pub use fixture::FixtureAdapter;
pub use registry::{AdapterRegistry, RegistryError};
