use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::cursor::Cursor;
use crate::dpi::{PhysicalPosition, PhysicalSize, Position, Size};
use crate::platform_impl::Fullscreen;
use crate::window::ImePurpose;
use crate::{error, window};

use super::event_loop::queue_redraw;
use super::{ActiveEventLoop, MonitorHandle, OsError, WindowId};

pub struct Window {
    toyos_window: Arc<Mutex<toyos_window::Window>>,
    id: WindowId,
    title: String,
    waker: toyos_window::Waker,
    redraws: Arc<Mutex<VecDeque<WindowId>>>,
    destroys: Arc<Mutex<VecDeque<WindowId>>>,
}

impl Window {
    pub(crate) fn new(
        el: &ActiveEventLoop,
        attrs: window::WindowAttributes,
    ) -> Result<Self, error::OsError> {
        // `0, 0` asks the compositor to choose.
        let (w, h): (u32, u32) = match attrs.inner_size {
            Some(size) => size.to_physical::<u32>(1.0).into(),
            None => (0, 0),
        };

        let toyos_window = toyos_window::Window::create_with_title(w, h, &attrs.title)
            .map_err(|e| os_error!(OsError(e)))?;
        let toyos_window = Arc::new(Mutex::new(toyos_window));
        let id = WindowId::next();

        el.creates.lock().unwrap().push_back((toyos_window.clone(), id));
        el.waker.wake();

        Ok(Self {
            toyos_window,
            id,
            title: attrs.title,
            waker: el.waker.clone(),
            redraws: el.redraws.clone(),
            destroys: el.destroys.clone(),
        })
    }

    pub(crate) fn maybe_queue_on_main(&self, f: impl FnOnce(&Self) + Send + 'static) {
        f(self)
    }

    pub(crate) fn maybe_wait_on_main<R: Send>(&self, f: impl FnOnce(&Self) -> R + Send) -> R {
        f(self)
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
    }

    #[inline]
    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        None
    }

    #[inline]
    pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
        VecDeque::new()
    }

    #[inline]
    pub fn current_monitor(&self) -> Option<MonitorHandle> {
        None
    }

    #[inline]
    pub fn scale_factor(&self) -> f64 {
        1.0
    }

    #[inline]
    pub fn request_redraw(&self) {
        queue_redraw(&self.redraws, self.id);
        self.waker.wake();
    }

    /// The surface that draws into the window presents it; there is nothing to
    /// do ahead of that.
    #[inline]
    pub fn pre_present_notify(&self) {}

    #[inline]
    pub fn reset_dead_keys(&self) {}

    #[inline]
    pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, error::NotSupportedError> {
        Err(error::NotSupportedError::new())
    }

    #[inline]
    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, error::NotSupportedError> {
        Err(error::NotSupportedError::new())
    }

    #[inline]
    pub fn set_outer_position(&self, _position: Position) {}

    #[inline]
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        let win = self.toyos_window.lock().unwrap();
        (win.width(), win.height()).into()
    }

    #[inline]
    pub fn request_inner_size(&self, _size: Size) -> Option<PhysicalSize<u32>> {
        None
    }

    #[inline]
    pub fn outer_size(&self) -> PhysicalSize<u32> {
        self.inner_size()
    }

    #[inline]
    pub fn set_min_inner_size(&self, _: Option<Size>) {}

    #[inline]
    pub fn set_max_inner_size(&self, _: Option<Size>) {}

    /// The title the window was created with; the compositor takes no other.
    #[inline]
    pub fn title(&self) -> String {
        self.title.clone()
    }

    #[inline]
    pub fn set_title(&self, _title: &str) {}

    #[inline]
    pub fn set_transparent(&self, _transparent: bool) {}

    #[inline]
    pub fn set_blur(&self, _blur: bool) {}

    #[inline]
    pub fn set_visible(&self, _visible: bool) {}

    #[inline]
    pub fn is_visible(&self) -> Option<bool> {
        Some(true)
    }

    #[inline]
    pub fn resize_increments(&self) -> Option<PhysicalSize<u32>> {
        None
    }

    #[inline]
    pub fn set_resize_increments(&self, _increments: Option<Size>) {}

    #[inline]
    pub fn set_resizable(&self, _resizable: bool) {}

    #[inline]
    pub fn is_resizable(&self) -> bool {
        true
    }

    #[inline]
    pub fn set_minimized(&self, _minimized: bool) {}

    #[inline]
    pub fn is_minimized(&self) -> Option<bool> {
        None
    }

    #[inline]
    pub fn set_maximized(&self, _maximized: bool) {}

    #[inline]
    pub fn is_maximized(&self) -> bool {
        false
    }

    #[inline]
    pub(crate) fn set_fullscreen(&self, _monitor: Option<Fullscreen>) {}

    #[inline]
    pub(crate) fn fullscreen(&self) -> Option<Fullscreen> {
        None
    }

    #[inline]
    pub fn set_decorations(&self, _decorations: bool) {}

    #[inline]
    pub fn is_decorated(&self) -> bool {
        true
    }

    #[inline]
    pub fn set_window_level(&self, _level: window::WindowLevel) {}

    #[inline]
    pub fn set_window_icon(&self, _window_icon: Option<crate::icon::Icon>) {}

    #[inline]
    pub fn set_ime_cursor_area(&self, _position: Position, _size: Size) {}

    #[inline]
    pub fn set_ime_allowed(&self, _allowed: bool) {}

    #[inline]
    pub fn set_ime_purpose(&self, _purpose: ImePurpose) {}

    #[inline]
    pub fn focus_window(&self) {}

    #[inline]
    pub fn request_user_attention(&self, _request_type: Option<window::UserAttentionType>) {}

    #[inline]
    pub fn set_cursor(&self, _: Cursor) {}

    #[inline]
    pub fn set_cursor_position(&self, _: Position) -> Result<(), error::ExternalError> {
        Err(error::ExternalError::NotSupported(error::NotSupportedError::new()))
    }

    #[inline]
    pub fn set_cursor_grab(&self, _mode: window::CursorGrabMode) -> Result<(), error::ExternalError> {
        Err(error::ExternalError::NotSupported(error::NotSupportedError::new()))
    }

    #[inline]
    pub fn set_cursor_visible(&self, _visible: bool) {}

    #[inline]
    pub fn drag_window(&self) -> Result<(), error::ExternalError> {
        Err(error::ExternalError::NotSupported(error::NotSupportedError::new()))
    }

    #[inline]
    pub fn drag_resize_window(
        &self,
        _direction: window::ResizeDirection,
    ) -> Result<(), error::ExternalError> {
        Err(error::ExternalError::NotSupported(error::NotSupportedError::new()))
    }

    #[inline]
    pub fn show_window_menu(&self, _position: Position) {}

    #[inline]
    pub fn set_cursor_hittest(&self, _hittest: bool) -> Result<(), error::ExternalError> {
        Err(error::ExternalError::NotSupported(error::NotSupportedError::new()))
    }

    /// The handle is the address of the `toyos_window::Window` inside this
    /// window's `Arc`, which does not move for as long as the window lives.
    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
        let ptr = &*self.toyos_window.lock().unwrap() as *const toyos_window::Window;
        let handle = rwh_06::ToyOsWindowHandle::new(
            std::ptr::NonNull::new(ptr as *mut std::ffi::c_void)
                .expect("a reference is never null"),
        );
        Ok(rwh_06::RawWindowHandle::ToyOs(handle))
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::RawDisplayHandle::ToyOs(rwh_06::ToyOsDisplayHandle::new()))
    }

    #[inline]
    pub fn set_enabled_buttons(&self, _buttons: window::WindowButtons) {}

    #[inline]
    pub fn enabled_buttons(&self) -> window::WindowButtons {
        window::WindowButtons::all()
    }

    #[inline]
    pub fn theme(&self) -> Option<window::Theme> {
        None
    }

    #[inline]
    pub fn has_focus(&self) -> bool {
        false
    }

    #[inline]
    pub fn set_theme(&self, _theme: Option<window::Theme>) {}

    pub fn set_content_protected(&self, _protected: bool) {}
}

impl Drop for Window {
    fn drop(&mut self) {
        self.destroys.lock().unwrap().push_back(self.id);
        self.waker.wake();
    }
}
