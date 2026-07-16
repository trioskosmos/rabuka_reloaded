#[cfg(feature = "psp")]
pub(crate) use hashbrown::{HashMap, HashSet};
/// Platform-compatibility re-exports.
/// Maps std types to their no_std equivalents when compiling for PSP.

#[cfg(not(feature = "psp"))]
pub(crate) use std::collections::{HashMap, HashSet};

#[cfg(feature = "psp")]
pub(crate) use alloc::sync::Arc;
#[cfg(not(feature = "psp"))]
pub(crate) use std::sync::Arc;

#[cfg(feature = "psp")]
pub(crate) use once_cell::sync::OnceCell as OnceLock;
#[cfg(not(feature = "psp"))]
pub(crate) use std::sync::OnceLock;

#[cfg(feature = "psp")]
pub(crate) use alloc::collections::VecDeque;
#[cfg(not(feature = "psp"))]
pub(crate) use std::collections::VecDeque;
