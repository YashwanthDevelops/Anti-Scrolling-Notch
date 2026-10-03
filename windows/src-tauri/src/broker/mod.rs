//! Backend-owned state contracts.
//!
//! These types describe normalized application data. They are not upstream
//! Codex event schemas and do not enable any adapter or capability.

pub mod reducer;
pub mod request_router;
pub mod stream;
pub mod types;
