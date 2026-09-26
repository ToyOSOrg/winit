use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::iter;

use bitflags::bitflags;
use smol_str::SmolStr;
use window as toyos_window;
use winit_core::application::ApplicationHandler;
use winit_core::cursor::{CustomCursor, CustomCursorSource};
use winit_core::error::{EventLoopError, NotSupportedError, RequestError};
use winit_core::event::{self, Modifiers, StartCause};
use winit_core::event_loop::{
    ActiveEventLoop as RootActiveEventLoop, ControlFlow, DeviceEvents,
    EventLoopProxy as CoreEventLoopProxy, EventLoopProxyProvider,
    OwnedDisplayHandle as CoreOwnedDisplayHandle,
};
use winit_core::keyboard::{
    Key, KeyCode, KeyLocation, ModifiersKeys, ModifiersState, NamedKey, NativeKey, NativeKeyCode,
    PhysicalKey,
};
use winit_core::window::{Theme, Window as CoreWindow, WindowId};

use crate::window::Window;

/// Convert a USB HID keycode to a winit PhysicalKey and optional NamedKey.
fn convert_hid_keycode(keycode: u8) -> (PhysicalKey, Option<NamedKey>) {
    let (key_code, named_key_opt) = match keycode {
        // Letters: HID 0x04-0x1D = A-Z
        0x04 => (KeyCode::KeyA, None),
        0x05 => (KeyCode::KeyB, None),
        0x06 => (KeyCode::KeyC, None),
        0x07 => (KeyCode::KeyD, None),
        0x08 => (KeyCode::KeyE, None),
        0x09 => (KeyCode::KeyF, None),
        0x0A => (KeyCode::KeyG, None),
        0x0B => (KeyCode::KeyH, None),
        0x0C => (KeyCode::KeyI, None),
        0x0D => (KeyCode::KeyJ, None),
        0x0E => (KeyCode::KeyK, None),
        0x0F => (KeyCode::KeyL, None),
        0x10 => (KeyCode::KeyM, None),
        0x11 => (KeyCode::KeyN, None),
        0x12 => (KeyCode::KeyO, None),
        0x13 => (KeyCode::KeyP, None),
        0x14 => (KeyCode::KeyQ, None),
        0x15 => (KeyCode::KeyR, None),
        0x16 => (KeyCode::KeyS, None),
        0x17 => (KeyCode::KeyT, None),
        0x18 => (KeyCode::KeyU, None),
        0x19 => (KeyCode::KeyV, None),
        0x1A => (KeyCode::KeyW, None),
        0x1B => (KeyCode::KeyX, None),
        0x1C => (KeyCode::KeyY, None),
        0x1D => (KeyCode::KeyZ, None),

        // Digits: HID 0x1E-0x27 = 1-9, 0
        0x1E => (KeyCode::Digit1, None),
        0x1F => (KeyCode::Digit2, None),
        0x20 => (KeyCode::Digit3, None),
        0x21 => (KeyCode::Digit4, None),
        0x22 => (KeyCode::Digit5, None),
        0x23 => (KeyCode::Digit6, None),
        0x24 => (KeyCode::Digit7, None),
        0x25 => (KeyCode::Digit8, None),
        0x26 => (KeyCode::Digit9, None),
        0x27 => (KeyCode::Digit0, None),

        // Special keys
        0x28 => (KeyCode::Enter, Some(NamedKey::Enter)),
        0x29 => (KeyCode::Escape, Some(NamedKey::Escape)),
        0x2A => (KeyCode::Backspace, Some(NamedKey::Backspace)),
        0x2B => (KeyCode::Tab, Some(NamedKey::Tab)),
        0x2C => (KeyCode::Space, None),

        // Symbols
        0x2D => (KeyCode::Minus, None),
        0x2E => (KeyCode::Equal, None),
        0x2F => (KeyCode::BracketLeft, None),
        0x30 => (KeyCode::BracketRight, None),
        0x31 => (KeyCode::Backslash, None),
        0x33 => (KeyCode::Semicolon, None),
        0x34 => (KeyCode::Quote, None),
        0x35 => (KeyCode::Backquote, None),
        0x36 => (KeyCode::Comma, None),
        0x37 => (KeyCode::Period, None),
        0x38 => (KeyCode::Slash, None),

        // Caps Lock
        0x39 => (KeyCode::CapsLock, Some(NamedKey::CapsLock)),

        // F1-F12
        0x3A => (KeyCode::F1, Some(NamedKey::F1)),
        0x3B => (KeyCode::F2, Some(NamedKey::F2)),
        0x3C => (KeyCode::F3, Some(NamedKey::F3)),
        0x3D => (KeyCode::F4, Some(NamedKey::F4)),
        0x3E => (KeyCode::F5, Some(NamedKey::F5)),
        0x3F => (KeyCode::F6, Some(NamedKey::F6)),
        0x40 => (KeyCode::F7, Some(NamedKey::F7)),
        0x41 => (KeyCode::F8, Some(NamedKey::F8)),
        0x42 => (KeyCode::F9, Some(NamedKey::F9)),
        0x43 => (KeyCode::F10, Some(NamedKey::F10)),
        0x44 => (KeyCode::F11, Some(NamedKey::F11)),
        0x45 => (KeyCode::F12, Some(NamedKey::F12)),

        // Print Screen, Scroll Lock, Pause
        0x46 => (KeyCode::PrintScreen, Some(NamedKey::PrintScreen)),
        0x47 => (KeyCode::ScrollLock, Some(NamedKey::ScrollLock)),
        0x48 => (KeyCode::Pause, Some(NamedKey::Pause)),

        // Insert, Home, Page Up, Delete, End, Page Down
        0x49 => (KeyCode::Insert, Some(NamedKey::Insert)),
        0x4A => (KeyCode::Home, Some(NamedKey::Home)),
        0x4B => (KeyCode::PageUp, Some(NamedKey::PageUp)),
        0x4C => (KeyCode::Delete, Some(NamedKey::Delete)),
        0x4D => (KeyCode::End, Some(NamedKey::End)),
        0x4E => (KeyCode::PageDown, Some(NamedKey::PageDown)),

        // Arrow keys
        0x4F => (KeyCode::ArrowRight, Some(NamedKey::ArrowRight)),
        0x50 => (KeyCode::ArrowLeft, Some(NamedKey::ArrowLeft)),
        0x51 => (KeyCode::ArrowDown, Some(NamedKey::ArrowDown)),
        0x52 => (KeyCode::ArrowUp, Some(NamedKey::ArrowUp)),

        // Numpad
        0x53 => (KeyCode::NumLock, Some(NamedKey::NumLock)),
        0x54 => (KeyCode::NumpadDivide, None),
        0x55 => (KeyCode::NumpadMultiply, None),
        0x56 => (KeyCode::NumpadSubtract, None),
        0x57 => (KeyCode::NumpadAdd, None),
        0x58 => (KeyCode::NumpadEnter, Some(NamedKey::Enter)),
        0x59 => (KeyCode::Numpad1, None),
        0x5A => (KeyCode::Numpad2, None),
        0x5B => (KeyCode::Numpad3, None),
        0x5C => (KeyCode::Numpad4, None),
        0x5D => (KeyCode::Numpad5, None),
        0x5E => (KeyCode::Numpad6, None),
        0x5F => (KeyCode::Numpad7, None),
        0x60 => (KeyCode::Numpad8, None),
        0x61 => (KeyCode::Numpad9, None),
        0x62 => (KeyCode::Numpad0, None),
        0x63 => (KeyCode::NumpadDecimal, None),

        // Modifier keys
        0xE0 => (KeyCode::ControlLeft, Some(NamedKey::Control)),
        0xE1 => (KeyCode::ShiftLeft, Some(NamedKey::Shift)),
        0xE2 => (KeyCode::AltLeft, Some(NamedKey::Alt)),
        0xE3 => (KeyCode::MetaLeft, Some(NamedKey::Meta)),
        0xE4 => (KeyCode::ControlRight, Some(NamedKey::Control)),
        0xE5 => (KeyCode::ShiftRight, Some(NamedKey::Shift)),
        0xE6 => (KeyCode::AltRight, Some(NamedKey::AltGraph)),
        0xE7 => (KeyCode::MetaRight, Some(NamedKey::Meta)),

        _ => return (PhysicalKey::Unidentified(NativeKeyCode::Unidentified), None),
    };
    (PhysicalKey::Code(key_code), named_key_opt)
}

/// Pops without holding the lock past the call: the handler the item goes to may push.
fn pop<T>(queue: &Mutex<VecDeque<T>>) -> Option<T> {
    queue.lock().unwrap().pop_front()
}

pub(crate) fn queue_redraw(redraws: &Mutex<VecDeque<WindowId>>, id: WindowId) {
    let mut redraws = redraws.lock().unwrap();
    if !redraws.contains(&id) {
        redraws.push_back(id);
    }
}

fn element_state(pressed: bool) -> event::ElementState {
    if pressed { event::ElementState::Pressed } else { event::ElementState::Released }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct KeyboardModifierState: u8 {
        const LSHIFT = 1 << 0;
        const RSHIFT = 1 << 1;
        const LCTRL = 1 << 2;
        const RCTRL = 1 << 3;
        const LALT = 1 << 4;
        const RALT = 1 << 5;
        const LMETA = 1 << 6;
        const RMETA = 1 << 7;
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct MouseButtonState: u8 {
        const LEFT = 1 << 0;
        const MIDDLE = 1 << 1;
        const RIGHT = 1 << 2;
    }
}

#[derive(Default, Debug)]
struct EventState {
    keyboard: KeyboardModifierState,
    mouse: MouseButtonState,
}

impl EventState {
    fn key(&mut self, key: PhysicalKey, pressed: bool) {
        let code = match key {
            PhysicalKey::Code(code) => code,
            _ => return,
        };

        match code {
            KeyCode::ShiftLeft => self.keyboard.set(KeyboardModifierState::LSHIFT, pressed),
            KeyCode::ShiftRight => self.keyboard.set(KeyboardModifierState::RSHIFT, pressed),
            KeyCode::ControlLeft => self.keyboard.set(KeyboardModifierState::LCTRL, pressed),
            KeyCode::ControlRight => self.keyboard.set(KeyboardModifierState::RCTRL, pressed),
            KeyCode::AltLeft => self.keyboard.set(KeyboardModifierState::LALT, pressed),
            KeyCode::AltRight => self.keyboard.set(KeyboardModifierState::RALT, pressed),
            KeyCode::MetaLeft => self.keyboard.set(KeyboardModifierState::LMETA, pressed),
            KeyCode::MetaRight => self.keyboard.set(KeyboardModifierState::RMETA, pressed),
            _ => (),
        }
    }

    fn modifiers(&self) -> Modifiers {
        let mut state = ModifiersState::empty();
        let mut pressed_mods = ModifiersKeys::empty();

        if self.keyboard.intersects(KeyboardModifierState::LSHIFT | KeyboardModifierState::RSHIFT) {
            state |= ModifiersState::SHIFT;
        }

        pressed_mods
            .set(ModifiersKeys::LSHIFT, self.keyboard.contains(KeyboardModifierState::LSHIFT));
        pressed_mods
            .set(ModifiersKeys::RSHIFT, self.keyboard.contains(KeyboardModifierState::RSHIFT));

        if self.keyboard.intersects(KeyboardModifierState::LCTRL | KeyboardModifierState::RCTRL) {
            state |= ModifiersState::CONTROL;
        }

        pressed_mods
            .set(ModifiersKeys::LCONTROL, self.keyboard.contains(KeyboardModifierState::LCTRL));
        pressed_mods
            .set(ModifiersKeys::RCONTROL, self.keyboard.contains(KeyboardModifierState::RCTRL));

        if self.keyboard.intersects(KeyboardModifierState::LALT | KeyboardModifierState::RALT) {
            state |= ModifiersState::ALT;
        }

        pressed_mods.set(ModifiersKeys::LALT, self.keyboard.contains(KeyboardModifierState::LALT));
        pressed_mods.set(ModifiersKeys::RALT, self.keyboard.contains(KeyboardModifierState::RALT));

        if self.keyboard.intersects(KeyboardModifierState::LMETA | KeyboardModifierState::RMETA) {
            state |= ModifiersState::META
        }

        pressed_mods
            .set(ModifiersKeys::LMETA, self.keyboard.contains(KeyboardModifierState::LMETA));
        pressed_mods
            .set(ModifiersKeys::RMETA, self.keyboard.contains(KeyboardModifierState::RMETA));

        Modifiers::new(state, pressed_mods)
    }
}

/// A window the loop has been told about and has not yet seen destroyed.
struct LiveWindow {
    id: WindowId,
    window: Arc<Mutex<toyos_window::Window>>,
    /// The window's connection, which the loop's waiter watches. Live for as
    /// long as `window` is, which this entry holds.
    handle: toyos_window::RawHandle,
    /// `Close` has been delivered: the connection is at its end and stays
    /// readable, so it is no longer waited on.
    closed: bool,
}

impl LiveWindow {
    /// The next event if one is ready, with the window unlocked again before it is handled.
    fn poll(&self) -> Option<toyos_window::Event> {
        self.window.lock().unwrap().poll_event(0)
    }
}

pub struct EventLoop {
    window_target: ActiveEventLoop,
    waiter: toyos_window::Waiter,
}

impl std::fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventLoop").finish_non_exhaustive()
    }
}

impl EventLoop {
    pub fn new(_: &PlatformSpecificEventLoopAttributes) -> Result<Self, EventLoopError> {
        static EVENT_LOOP_CREATED: AtomicBool = AtomicBool::new(false);
        if EVENT_LOOP_CREATED.swap(true, Ordering::Relaxed) {
            return Err(EventLoopError::RecreationAttempt);
        }

        let waiter = toyos_window::Waiter::new();

        Ok(Self {
            window_target: ActiveEventLoop {
                control_flow: Cell::new(ControlFlow::default()),
                exit: Cell::new(false),
                waker: waiter.waker(),
                creates: Mutex::new(VecDeque::new()),
                redraws: Arc::new(Mutex::new(VecDeque::new())),
                destroys: Arc::new(Mutex::new(VecDeque::new())),
                event_loop_proxy: Arc::new(EventLoopProxy {
                    woken: AtomicBool::new(false),
                    waker: waiter.waker(),
                }),
            },
            waiter,
        })
    }

    fn process_key_event<A: ApplicationHandler>(
        window_id: WindowId,
        key_event: toyos_window::KeyPress,
        event_state: &mut EventState,
        window_target: &ActiveEventLoop,
        app: &mut A,
    ) {
        let pressed = key_event.pressed();
        let (physical_key, named_key_opt) = convert_hid_keycode(key_event.keycode);

        let modifiers_before = event_state.keyboard;
        event_state.key(physical_key, pressed);

        // Build logical key from the characters this press types, which the
        // window resolved under the layout in force.
        let mut logical_key = Key::Unidentified(NativeKey::Unidentified);
        let mut key_without_modifiers = logical_key.clone();
        let mut text = None;
        let mut text_with_all_modifiers = None;

        let typed = key_event.text();
        if !typed.is_empty() {
            logical_key = Key::Character(SmolStr::new(typed));
            key_without_modifiers =
                Key::Character(SmolStr::from_iter(typed.chars().flat_map(|c| c.to_lowercase())));
            if pressed {
                text = Some(SmolStr::new(typed));
                text_with_all_modifiers = Some(SmolStr::new(typed));
            }
        }

        if let Some(named_key) = named_key_opt {
            logical_key = Key::Named(named_key);
            key_without_modifiers = logical_key.clone();
        }

        let event = event::WindowEvent::KeyboardInput {
            device_id: None,
            event: event::KeyEvent {
                logical_key,
                physical_key,
                location: KeyLocation::Standard,
                state: element_state(pressed),
                repeat: false,
                text,
                key_without_modifiers,
                text_with_all_modifiers,
            },
            is_synthetic: false,
        };

        app.window_event(window_target, window_id, event);

        if modifiers_before != event_state.keyboard {
            app.window_event(
                window_target,
                window_id,
                event::WindowEvent::ModifiersChanged(event_state.modifiers()),
            );
        }
    }

    fn process_mouse_event<A: ApplicationHandler>(
        window_id: WindowId,
        mouse: toyos_window::MouseEvent,
        event_state: &mut EventState,
        window_target: &ActiveEventLoop,
        app: &mut A,
    ) {
        match mouse.event_type {
            toyos_window::MOUSE_MOVE => {
                app.window_event(window_target, window_id, event::WindowEvent::PointerMoved {
                    device_id: None,
                    primary: true,
                    position: (mouse.x as f64, mouse.y as f64).into(),
                    source: event::PointerSource::Mouse,
                });
            },
            toyos_window::MOUSE_PRESS => {
                let button = match mouse.changed {
                    0x01 => event::MouseButton::Left,
                    0x02 => event::MouseButton::Right,
                    0x04 => event::MouseButton::Middle,
                    _ => event::MouseButton::Left,
                };

                // Track button state.
                if mouse.changed & 0x01 != 0 {
                    event_state.mouse.set(MouseButtonState::LEFT, true);
                }
                if mouse.changed & 0x02 != 0 {
                    event_state.mouse.set(MouseButtonState::RIGHT, true);
                }
                if mouse.changed & 0x04 != 0 {
                    event_state.mouse.set(MouseButtonState::MIDDLE, true);
                }

                app.window_event(window_target, window_id, event::WindowEvent::PointerButton {
                    device_id: None,
                    primary: true,
                    state: event::ElementState::Pressed,
                    position: (mouse.x as f64, mouse.y as f64).into(),
                    button: button.into(),
                });
            },
            toyos_window::MOUSE_RELEASE => {
                let button = match mouse.changed {
                    0x01 => event::MouseButton::Left,
                    0x02 => event::MouseButton::Right,
                    0x04 => event::MouseButton::Middle,
                    _ => event::MouseButton::Left,
                };

                // Track button state.
                if mouse.changed & 0x01 != 0 {
                    event_state.mouse.set(MouseButtonState::LEFT, false);
                }
                if mouse.changed & 0x02 != 0 {
                    event_state.mouse.set(MouseButtonState::RIGHT, false);
                }
                if mouse.changed & 0x04 != 0 {
                    event_state.mouse.set(MouseButtonState::MIDDLE, false);
                }

                app.window_event(window_target, window_id, event::WindowEvent::PointerButton {
                    device_id: None,
                    primary: true,
                    state: event::ElementState::Released,
                    position: (mouse.x as f64, mouse.y as f64).into(),
                    button: button.into(),
                });
            },
            toyos_window::MOUSE_SCROLL => {
                app.window_event(window_target, window_id, event::WindowEvent::MouseWheel {
                    device_id: None,
                    delta: event::MouseScrollDelta::LineDelta(0.0, mouse.scroll as f32),
                    phase: event::TouchPhase::Moved,
                });
            },
            _ => {},
        }
    }

    fn process_event<A: ApplicationHandler>(
        live: &mut LiveWindow,
        toyos_event: toyos_window::Event,
        event_state: &mut EventState,
        window_target: &ActiveEventLoop,
        app: &mut A,
    ) {
        let wid = live.id;
        match toyos_event {
            toyos_window::Event::KeyInput(key_event) => {
                let press = live.window.lock().unwrap().press(key_event);
                Self::process_key_event(wid, press, event_state, window_target, app);
            },
            toyos_window::Event::MouseInput(mouse_event) => {
                Self::process_mouse_event(wid, mouse_event, event_state, window_target, app);
            },
            toyos_window::Event::Resized => {
                let (w, h) = {
                    let w = live.window.lock().unwrap();
                    (w.width(), w.height())
                };
                app.window_event(
                    window_target,
                    wid,
                    event::WindowEvent::SurfaceResized((w, h).into()),
                );
                queue_redraw(&window_target.redraws, wid);
            },
            toyos_window::Event::Close => {
                live.closed = true;
                app.window_event(window_target, wid, event::WindowEvent::CloseRequested);
            },
            // The compositor is ready for the next frame.
            toyos_window::Event::Frame => queue_redraw(&window_target.redraws, wid),
            // Clipboard paste events are not directly mapped to winit events.
            toyos_window::Event::ClipboardPaste(_) => {},
            // The window has already re-read the layout; what a key types
            // comes from it on the next press.
            toyos_window::Event::LayoutChanged => {},
        }
    }

    /// Work the loop already has, which no wait may hold back: a redraw
    /// requested from `about_to_wait`, or a window created or dropped in a
    /// handler after its queue was drained, which cannot be waited on until it
    /// is.
    fn has_pending(&self) -> bool {
        let target = &self.window_target;
        !target.redraws.lock().unwrap().is_empty()
            || !target.creates.lock().unwrap().is_empty()
            || !target.destroys.lock().unwrap().is_empty()
    }

    pub fn run_app_on_demand<A: ApplicationHandler>(
        &mut self,
        mut app: A,
    ) -> Result<(), EventLoopError> {
        let mut start_cause = StartCause::Init;
        let mut event_state = EventState::default();
        let mut windows: Vec<LiveWindow> = Vec::new();

        loop {
            // Taken before anything a wake announces is read, so a wake raised
            // after this is still pending at the next wait.
            self.waiter.take_wake();

            app.new_events(&self.window_target, start_cause);

            if start_cause == StartCause::Init {
                app.can_create_surfaces(&self.window_target);
            }

            while let Some((window, id)) = pop(&self.window_target.creates) {
                let (handle, size) = {
                    let w = window.lock().unwrap();
                    (w.handle(), (w.width(), w.height()))
                };
                windows.push(LiveWindow { id, window, handle, closed: false });
                app.window_event(
                    &self.window_target,
                    id,
                    event::WindowEvent::SurfaceResized(size.into()),
                );
            }

            while let Some(id) = pop(&self.window_target.destroys) {
                app.window_event(&self.window_target, id, event::WindowEvent::Destroyed);
                windows.retain(|live| live.id != id);
            }

            for live in &mut windows {
                // `poll_event` answers `None` for good once it has handed out `Close`.
                while let Some(toyos_event) = live.poll() {
                    Self::process_event(
                        live,
                        toyos_event,
                        &mut event_state,
                        &self.window_target,
                        &mut app,
                    );
                }
            }

            if self.window_target.event_loop_proxy.woken.swap(false, Ordering::Acquire) {
                app.proxy_wake_up(&self.window_target);
            }

            while let Some(window_id) = pop(&self.window_target.redraws) {
                app.window_event(
                    &self.window_target,
                    window_id,
                    event::WindowEvent::RedrawRequested,
                );
            }

            app.about_to_wait(&self.window_target);

            if self.window_target.exiting() {
                break;
            }

            if self.has_pending() {
                start_cause = StartCause::Poll;
                continue;
            }

            let start = Instant::now();
            let timeout = match self.window_target.control_flow() {
                ControlFlow::Poll => {
                    start_cause = StartCause::Poll;
                    continue;
                },
                ControlFlow::Wait => None,
                ControlFlow::WaitUntil(deadline) => Some(deadline.saturating_duration_since(start)),
            };

            let open = windows.iter().filter(|live| !live.closed).map(|live| live.handle);
            self.waiter.wait(open, timeout);

            start_cause = match self.window_target.control_flow() {
                ControlFlow::WaitUntil(deadline) if Instant::now() >= deadline => {
                    StartCause::ResumeTimeReached { start, requested_resume: deadline }
                },
                ControlFlow::WaitUntil(deadline) => {
                    StartCause::WaitCancelled { start, requested_resume: Some(deadline) }
                },
                _ => StartCause::WaitCancelled { start, requested_resume: None },
            };
        }

        Ok(())
    }

    pub fn window_target(&self) -> &dyn RootActiveEventLoop {
        &self.window_target
    }
}

#[derive(Debug)]
pub struct EventLoopProxy {
    /// A `wake_up` the loop has not yet handed to `proxy_wake_up`. Set before
    /// the waker is raised, and taken after the loop takes its wakes.
    woken: AtomicBool,
    waker: toyos_window::Waker,
}

impl EventLoopProxyProvider for EventLoopProxy {
    fn wake_up(&self) {
        self.woken.store(true, Ordering::Release);
        self.waker.wake();
    }
}

impl Unpin for EventLoopProxy {}

pub struct ActiveEventLoop {
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
    /// Ends the loop's wait; every queue below is pushed and then woken, since
    /// a window may be created, redrawn or dropped from any thread.
    pub(super) waker: toyos_window::Waker,
    pub(super) creates: Mutex<VecDeque<(Arc<Mutex<toyos_window::Window>>, WindowId)>>,
    pub(super) redraws: Arc<Mutex<VecDeque<WindowId>>>,
    pub(super) destroys: Arc<Mutex<VecDeque<WindowId>>>,
    pub(super) event_loop_proxy: Arc<EventLoopProxy>,
}

impl std::fmt::Debug for ActiveEventLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveEventLoop")
            .field("control_flow", &self.control_flow)
            .field("exit", &self.exit)
            .finish_non_exhaustive()
    }
}

impl RootActiveEventLoop for ActiveEventLoop {
    fn create_proxy(&self) -> CoreEventLoopProxy {
        CoreEventLoopProxy::new(self.event_loop_proxy.clone())
    }

    fn create_window(
        &self,
        window_attributes: winit_core::window::WindowAttributes,
    ) -> Result<Box<dyn CoreWindow>, RequestError> {
        Ok(Box::new(Window::new(self, window_attributes)?))
    }

    fn create_custom_cursor(&self, _: CustomCursorSource) -> Result<CustomCursor, RequestError> {
        Err(NotSupportedError::new("create_custom_cursor is not supported").into())
    }

    fn available_monitors(&self) -> Box<dyn Iterator<Item = winit_core::monitor::MonitorHandle>> {
        Box::new(iter::empty())
    }

    fn system_theme(&self) -> Option<Theme> {
        None
    }

    fn primary_monitor(&self) -> Option<winit_core::monitor::MonitorHandle> {
        None
    }

    fn listen_device_events(&self, _allowed: DeviceEvents) {}

    fn set_control_flow(&self, control_flow: ControlFlow) {
        self.control_flow.set(control_flow)
    }

    fn control_flow(&self) -> ControlFlow {
        self.control_flow.get()
    }

    fn exit(&self) {
        self.exit.set(true);
    }

    fn exiting(&self) -> bool {
        self.exit.get()
    }

    fn owned_display_handle(&self) -> CoreOwnedDisplayHandle {
        CoreOwnedDisplayHandle::new(Arc::new(OwnedDisplayHandle))
    }

    fn rwh_06_handle(&self) -> &dyn rwh_06::HasDisplayHandle {
        self
    }
}

impl rwh_06::HasDisplayHandle for ActiveEventLoop {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = rwh_06::RawDisplayHandle::ToyOs(rwh_06::ToyOsDisplayHandle::new());
        unsafe { Ok(rwh_06::DisplayHandle::borrow_raw(raw)) }
    }
}

#[derive(Clone)]
pub(crate) struct OwnedDisplayHandle;

impl rwh_06::HasDisplayHandle for OwnedDisplayHandle {
    fn display_handle(&self) -> Result<rwh_06::DisplayHandle<'_>, rwh_06::HandleError> {
        let raw = rwh_06::RawDisplayHandle::ToyOs(rwh_06::ToyOsDisplayHandle::new());
        unsafe { Ok(rwh_06::DisplayHandle::borrow_raw(raw)) }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PlatformSpecificEventLoopAttributes {}
