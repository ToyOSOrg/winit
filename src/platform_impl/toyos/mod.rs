//! ToyOS backend: one compositor connection per window, through `toyos-window`.
//!
//! ToyOS has no monitors, IME, cursor control or fullscreen to offer a client;
//! those surfaces answer `None`/`NotSupported` rather than invent a value.

#![cfg(target_os = "toyos")]

#[cfg(any(feature = "rwh_04", feature = "rwh_05"))]
compile_error!("ToyOS has a raw-window-handle only in rwh_06; disable the rwh_04/rwh_05 features");

use std::convert::Infallible;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

use smol_str::SmolStr;

use crate::dpi::{PhysicalPosition, PhysicalSize};
use crate::keyboard::Key;

pub(crate) use self::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy, OwnedDisplayHandle};
mod event_loop;

pub use self::window::Window;
mod window;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlatformSpecificEventLoopAttributes {}

/// Minted from a process-wide counter: a window's identity is never an address
/// that a later window could come to occupy.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(u64);

impl WindowId {
    pub const fn dummy() -> Self {
        WindowId(u64::MAX)
    }

    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        WindowId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<WindowId> for u64 {
    fn from(id: WindowId) -> Self {
        id.0
    }
}

impl From<u64> for WindowId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceId;

impl DeviceId {
    pub const fn dummy() -> Self {
        DeviceId
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlatformSpecificWindowAttributes;

#[derive(Clone, Debug)]
pub struct OsError(toyos_window::CreateError);

impl Display for OsError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(fmt)
    }
}

pub(crate) use crate::cursor::{
    NoCustomCursor as PlatformCustomCursor, NoCustomCursor as PlatformCustomCursorSource,
};
pub(crate) use crate::icon::NoIcon as PlatformIcon;

/// Never constructed: the compositor tells a client nothing about the screens behind it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonitorHandle {
    never: Infallible,
}

impl MonitorHandle {
    pub fn name(&self) -> Option<String> {
        match self.never {}
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        match self.never {}
    }

    pub fn position(&self) -> PhysicalPosition<i32> {
        match self.never {}
    }

    pub fn scale_factor(&self) -> f64 {
        match self.never {}
    }

    pub fn refresh_rate_millihertz(&self) -> Option<u32> {
        match self.never {}
    }

    pub fn video_modes(&self) -> std::iter::Empty<VideoModeHandle> {
        match self.never {}
    }
}

/// Never constructed, because [`MonitorHandle`] is not.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VideoModeHandle {
    never: Infallible,
}

impl VideoModeHandle {
    pub fn size(&self) -> PhysicalSize<u32> {
        match self.never {}
    }

    pub fn bit_depth(&self) -> u16 {
        match self.never {}
    }

    pub fn refresh_rate_millihertz(&self) -> u32 {
        match self.never {}
    }

    pub fn monitor(&self) -> MonitorHandle {
        match self.never {}
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
    pub key_without_modifiers: Key,
    pub text_with_all_modifiers: Option<SmolStr>,
}

