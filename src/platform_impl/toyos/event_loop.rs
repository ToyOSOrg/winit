//! The ToyOS event loop: every window is its own compositor connection, and one
//! [`toyos_window::Waiter`] waits on all of them and on the loop's proxies at
//! once.

use std::cell::Cell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use bitflags::bitflags;
use smol_str::SmolStr;

use crate::dpi::PhysicalSize;
use crate::error::EventLoopError;
use crate::event::{self, Modifiers, StartCause};
use crate::event_loop::{self, ControlFlow, DeviceEvents};
use crate::keyboard::{
    Key, KeyCode, KeyLocation, ModifiersKeys, ModifiersState, NamedKey, NativeKey, NativeKeyCode,
    PhysicalKey,
};
use crate::platform::pump_events::PumpStatus;
use crate::window::{
    CustomCursor as RootCustomCursor, CustomCursorSource, Theme, WindowId as RootWindowId,
};

use super::{
    DeviceId, KeyEventExtra, MonitorHandle, PlatformSpecificEventLoopAttributes, WindowId,
};


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
        0xE3 => (KeyCode::SuperLeft, Some(NamedKey::Super)),
        0xE4 => (KeyCode::ControlRight, Some(NamedKey::Control)),
        0xE5 => (KeyCode::ShiftRight, Some(NamedKey::Shift)),
        0xE6 => (KeyCode::AltRight, Some(NamedKey::AltGraph)),
        0xE7 => (KeyCode::SuperRight, Some(NamedKey::Super)),

        _ => return (PhysicalKey::Unidentified(NativeKeyCode::Unidentified), None),
    };
    (PhysicalKey::Code(key_code), named_key_opt)
}

fn element_state(pressed: bool) -> event::ElementState {
    if pressed {
        event::ElementState::Pressed
    } else {
        event::ElementState::Released
    }
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
        const LSUPER = 1 << 6;
        const RSUPER = 1 << 7;
    }
}

impl KeyboardModifierState {
    fn key(&mut self, key: PhysicalKey, pressed: bool) {
        let flag = match key {
            PhysicalKey::Code(KeyCode::ShiftLeft) => Self::LSHIFT,
            PhysicalKey::Code(KeyCode::ShiftRight) => Self::RSHIFT,
            PhysicalKey::Code(KeyCode::ControlLeft) => Self::LCTRL,
            PhysicalKey::Code(KeyCode::ControlRight) => Self::RCTRL,
            PhysicalKey::Code(KeyCode::AltLeft) => Self::LALT,
            PhysicalKey::Code(KeyCode::AltRight) => Self::RALT,
            PhysicalKey::Code(KeyCode::SuperLeft) => Self::LSUPER,
            PhysicalKey::Code(KeyCode::SuperRight) => Self::RSUPER,
            _ => return,
        };
        self.set(flag, pressed);
    }

    fn modifiers(self) -> Modifiers {
        let mut state = ModifiersState::empty();
        state.set(ModifiersState::SHIFT, self.intersects(Self::LSHIFT | Self::RSHIFT));
        state.set(ModifiersState::CONTROL, self.intersects(Self::LCTRL | Self::RCTRL));
        state.set(ModifiersState::ALT, self.intersects(Self::LALT | Self::RALT));
        state.set(ModifiersState::SUPER, self.intersects(Self::LSUPER | Self::RSUPER));

        let mut pressed_mods = ModifiersKeys::empty();
        pressed_mods.set(ModifiersKeys::LSHIFT, self.contains(Self::LSHIFT));
        pressed_mods.set(ModifiersKeys::RSHIFT, self.contains(Self::RSHIFT));
        pressed_mods.set(ModifiersKeys::LCONTROL, self.contains(Self::LCTRL));
        pressed_mods.set(ModifiersKeys::RCONTROL, self.contains(Self::RCTRL));
        pressed_mods.set(ModifiersKeys::LALT, self.contains(Self::LALT));
        pressed_mods.set(ModifiersKeys::RALT, self.contains(Self::RALT));
        pressed_mods.set(ModifiersKeys::LSUPER, self.contains(Self::LSUPER));
        pressed_mods.set(ModifiersKeys::RSUPER, self.contains(Self::RSUPER));

        Modifiers { state, pressed_mods }
    }
}

/// The wire's button bits, in the order a press of several is reported.
const MOUSE_BUTTONS: [(u8, event::MouseButton); 3] = [
    (0x01, event::MouseButton::Left),
    (0x02, event::MouseButton::Right),
    (0x04, event::MouseButton::Middle),
];

/// A window the loop has been told about and has not yet seen destroyed.
struct LiveWindow {
    id: WindowId,
    window: Arc<Mutex<toyos_window::Window>>,
    /// The window's connection, which the loop's waiter watches. Live for as
    /// long as `window` is, which this entry holds.
    handle: toyos_window::RawHandle,
    keyboard: KeyboardModifierState,
    /// `Close` has been delivered: the connection is at its end and stays
    /// readable, so it is no longer waited on.
    closed: bool,
}

impl LiveWindow {
    fn size(&self) -> PhysicalSize<u32> {
        let w = self.window.lock().unwrap();
        PhysicalSize::new(w.width(), w.height())
    }

    /// The next event if one is ready, with the window unlocked again before it is handled.
    fn poll(&self) -> Option<toyos_window::Event> {
        self.window.lock().unwrap().poll_event(0)
    }

    fn process_event<T: 'static>(
        &mut self,
        toyos_event: toyos_window::Event,
        redraws: &Mutex<Redraws>,
        mut event_handler: impl FnMut(event::Event<T>),
    ) {
        let window_id = RootWindowId(self.id);
        let mut send = |event| event_handler(event::Event::WindowEvent { window_id, event });
        let device_id = event::DeviceId(DeviceId);
        match toyos_event {
            toyos_window::Event::KeyInput(key) => {
                let press = self.window.lock().unwrap().press(key);
                let pressed = press.pressed();
                let (physical_key, named_key) = convert_hid_keycode(press.keycode);

                let modifiers_before = self.keyboard;
                self.keyboard.key(physical_key, pressed);

                // What the press types, as the window resolved it under the
                // layout in force.
                let mut logical_key = Key::Unidentified(NativeKey::Unidentified);
                let mut key_without_modifiers = logical_key.clone();
                let mut text = None;
                let typed = press.text();
                if !typed.is_empty() {
                    logical_key = Key::Character(SmolStr::new(typed));
                    key_without_modifiers = Key::Character(SmolStr::from_iter(
                        typed.chars().flat_map(|c| c.to_lowercase()),
                    ));
                    if pressed {
                        text = Some(SmolStr::new(typed));
                    }
                }
                if let Some(named_key) = named_key {
                    logical_key = Key::Named(named_key);
                    key_without_modifiers = logical_key.clone();
                }

                send(event::WindowEvent::KeyboardInput {
                    device_id,
                    event: event::KeyEvent {
                        logical_key,
                        physical_key,
                        location: KeyLocation::Standard,
                        state: element_state(pressed),
                        repeat: false,
                        text: text.clone(),
                        platform_specific: KeyEventExtra {
                            key_without_modifiers,
                            text_with_all_modifiers: text,
                        },
                    },
                    is_synthetic: false,
                });

                if modifiers_before != self.keyboard {
                    send(event::WindowEvent::ModifiersChanged(self.keyboard.modifiers()));
                }
            },
            toyos_window::Event::MouseInput(mouse) => match mouse.event_type {
                toyos_window::MOUSE_MOVE => send(event::WindowEvent::CursorMoved {
                    device_id,
                    position: (mouse.x as f64, mouse.y as f64).into(),
                }),
                toyos_window::MOUSE_PRESS | toyos_window::MOUSE_RELEASE => {
                    let state = element_state(mouse.event_type == toyos_window::MOUSE_PRESS);
                    for (bit, button) in MOUSE_BUTTONS {
                        if mouse.changed & bit != 0 {
                            send(event::WindowEvent::MouseInput { device_id, state, button });
                        }
                    }
                },
                toyos_window::MOUSE_SCROLL => send(event::WindowEvent::MouseWheel {
                    device_id,
                    delta: event::MouseScrollDelta::LineDelta(0.0, mouse.scroll as f32),
                    phase: event::TouchPhase::Moved,
                }),
                other => tracing::warn!("toyos: unknown mouse event type {other}"),
            },
            toyos_window::Event::Resized => {
                send(event::WindowEvent::Resized(self.size()));
                redraws.lock().unwrap().request(self.id);
            },
            // The last present reached the panel. A redraw is the application's
            // to ask for; one it asked for since `pre_present_notify` goes now.
            toyos_window::Event::Frame => redraws.lock().unwrap().frame(self.id),
            toyos_window::Event::Close => {
                self.closed = true;
                send(event::WindowEvent::CloseRequested);
            },
            // Not a winit event; clipboard access is outside winit's API.
            toyos_window::Event::ClipboardPaste(_) => {},
            // The window re-read the layout itself; the next press types under it.
            toyos_window::Event::LayoutChanged => {},
        }
    }
}

/// Pops without holding the lock past the call: the handler the item goes to may push.
fn pop<T>(queue: &Mutex<VecDeque<T>>) -> Option<T> {
    queue.lock().unwrap().pop_front()
}

fn pop_redraw(redraws: &Mutex<Redraws>) -> Option<WindowId> {
    redraws.lock().unwrap().pop()
}

/// Whether the caller is somewhere other than the loop's thread, where a queue
/// it pushes is not seen until the loop's wait is woken. On the loop's thread
/// the push is work [`EventLoop::has_pending`] already finds.
pub(super) fn off_loop(loop_thread: ThreadId) -> bool {
    thread::current().id() != loop_thread
}

/// The redraws the loop owes, and the ones held for a frame event.
///
/// A window that called `pre_present_notify` has a present on its way to the
/// panel. A redraw asked of it before the compositor's frame event says that
/// present arrived is held until the event, which is what paces an
/// application that asks for its next frame from inside `RedrawRequested`.
#[derive(Default)]
pub(super) struct Redraws {
    queued: VecDeque<WindowId>,
    presenting: Vec<WindowId>,
    held: Vec<WindowId>,
}

impl Redraws {
    /// Queue a redraw, or hold it for the window's frame event: `true` when it
    /// was queued, which is when the loop has new work.
    pub(super) fn request(&mut self, id: WindowId) -> bool {
        if self.presenting.contains(&id) {
            if !self.held.contains(&id) {
                self.held.push(id);
            }
            return false;
        }
        if !self.queued.contains(&id) {
            self.queued.push_back(id);
        }
        true
    }

    pub(super) fn presenting(&mut self, id: WindowId) {
        if !self.presenting.contains(&id) {
            self.presenting.push(id);
        }
    }

    fn frame(&mut self, id: WindowId) {
        self.presenting.retain(|p| *p != id);
        if let Some(at) = self.held.iter().position(|h| *h == id) {
            self.held.swap_remove(at);
            self.request(id);
        }
    }

    /// Nothing is delivered for a window once it is dropped.
    pub(super) fn forget(&mut self, id: WindowId) {
        self.queued.retain(|q| *q != id);
        self.presenting.retain(|p| *p != id);
        self.held.retain(|h| *h != id);
    }

    fn pop(&mut self) -> Option<WindowId> {
        self.queued.pop_front()
    }
}

fn min_timeout(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    a.map_or(b, |a_timeout| b.map_or(Some(a_timeout), |b_timeout| Some(a_timeout.min(b_timeout))))
}

pub struct EventLoop<T: 'static> {
    windows: Vec<LiveWindow>,
    waiter: toyos_window::Waiter,
    window_target: event_loop::ActiveEventLoop,
    user_events_sender: mpsc::Sender<T>,
    user_events_receiver: mpsc::Receiver<T>,
    /// The `Init` iteration has run and `LoopExiting` has not.
    loop_running: bool,
}

impl<T: 'static> EventLoop<T> {
    pub(crate) fn new(_: &PlatformSpecificEventLoopAttributes) -> Result<Self, EventLoopError> {
        let (user_events_sender, user_events_receiver) = mpsc::channel();
        let waiter = toyos_window::Waiter::new();
        Ok(Self {
            windows: Vec::new(),
            window_target: event_loop::ActiveEventLoop {
                p: ActiveEventLoop {
                    control_flow: Cell::new(ControlFlow::default()),
                    exit: Cell::new(false),
                    waker: waiter.waker(),
                    loop_thread: thread::current().id(),
                    creates: Mutex::new(VecDeque::new()),
                    redraws: Arc::new(Mutex::new(Redraws::default())),
                    destroys: Arc::new(Mutex::new(VecDeque::new())),
                },
                _marker: PhantomData,
            },
            waiter,
            user_events_sender,
            user_events_receiver,
            loop_running: false,
        })
    }

    pub fn run<F>(mut self, event_handler: F) -> Result<(), EventLoopError>
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        self.run_on_demand(event_handler)
    }

    pub fn run_on_demand<F>(&mut self, mut event_handler: F) -> Result<(), EventLoopError>
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        loop {
            match self.pump_events(None, &mut event_handler) {
                PumpStatus::Exit(0) => break Ok(()),
                PumpStatus::Exit(code) => break Err(EventLoopError::ExitFailure(code)),
                PumpStatus::Continue => continue,
            }
        }
    }

    pub fn pump_events<F>(&mut self, timeout: Option<Duration>, mut callback: F) -> PumpStatus
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        if !self.loop_running {
            self.loop_running = true;
            self.single_iteration(&mut callback, StartCause::Init);
        }

        // The `Init` iteration may itself have asked to exit.
        if !self.window_target.p.exiting() {
            self.poll_events_with_timeout(timeout, &mut callback);
        }

        if self.window_target.p.exiting() {
            self.loop_running = false;
            callback(event::Event::LoopExiting, &self.window_target);
            PumpStatus::Exit(0)
        } else {
            PumpStatus::Continue
        }
    }

    /// Work the loop already has, which no wait may hold back: a redraw
    /// requested from `AboutToWait`, or a window created or dropped in a handler
    /// after its queue was drained, which cannot be waited on until it is.
    fn has_pending(&self) -> bool {
        let p = &self.window_target.p;
        !p.redraws.lock().unwrap().queued.is_empty()
            || !p.creates.lock().unwrap().is_empty()
            || !p.destroys.lock().unwrap().is_empty()
    }

    fn poll_events_with_timeout<F>(&mut self, timeout: Option<Duration>, callback: &mut F)
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        let start = Instant::now();

        let timeout = if self.has_pending() {
            Some(Duration::ZERO)
        } else {
            let control_flow_timeout = match self.window_target.p.control_flow() {
                ControlFlow::Wait => None,
                ControlFlow::Poll => Some(Duration::ZERO),
                ControlFlow::WaitUntil(deadline) => Some(deadline.saturating_duration_since(start)),
            };
            min_timeout(control_flow_timeout, timeout)
        };

        let open = self.windows.iter().filter(|live| !live.closed).map(|live| live.handle);
        self.waiter.wait(open, timeout);

        let cause = match self.window_target.p.control_flow() {
            ControlFlow::Poll => StartCause::Poll,
            ControlFlow::Wait => StartCause::WaitCancelled { start, requested_resume: None },
            ControlFlow::WaitUntil(deadline) => {
                if Instant::now() < deadline {
                    StartCause::WaitCancelled { start, requested_resume: Some(deadline) }
                } else {
                    StartCause::ResumeTimeReached { start, requested_resume: deadline }
                }
            },
        };

        self.single_iteration(callback, cause);
    }

    fn single_iteration<F>(&mut self, callback: &mut F, cause: StartCause)
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        let target = &self.window_target;

        // Taken before any queue a waker announces is read, so a wake raised
        // after this is still pending at the next wait.
        self.waiter.take_wake();

        callback(event::Event::NewEvents(cause), target);

        if cause == StartCause::Init {
            callback(event::Event::Resumed, target);
        }

        while let Some((window, id)) = pop(&target.p.creates) {
            let handle = window.lock().unwrap().handle();
            let live = LiveWindow {
                id,
                window,
                handle,
                keyboard: KeyboardModifierState::default(),
                closed: false,
            };
            let size = live.size();
            self.windows.push(live);
            callback(
                event::Event::WindowEvent {
                    window_id: RootWindowId(id),
                    event: event::WindowEvent::Resized(size),
                },
                target,
            );
            // A new window has never been drawn, as an exposed one has not.
            target.p.redraws.lock().unwrap().request(id);
        }

        while let Some(id) = pop(&target.p.destroys) {
            callback(
                event::Event::WindowEvent {
                    window_id: RootWindowId(id),
                    event: event::WindowEvent::Destroyed,
                },
                target,
            );
            self.windows.retain(|live| live.id != id);
            // The redraw the create above queued, when the window went in the
            // same handler it came in.
            target.p.redraws.lock().unwrap().forget(id);
        }

        for live in &mut self.windows {
            // `poll_event` answers `None` for good once it has handed out `Close`.
            while let Some(toyos_event) = live.poll() {
                live.process_event(toyos_event, &target.p.redraws, |event| callback(event, target));
            }
        }

        while let Ok(user_event) = self.user_events_receiver.try_recv() {
            callback(event::Event::UserEvent(user_event), target);
        }

        while let Some(id) = pop_redraw(&target.p.redraws) {
            callback(
                event::Event::WindowEvent {
                    window_id: RootWindowId(id),
                    event: event::WindowEvent::RedrawRequested,
                },
                target,
            );
        }

        callback(event::Event::AboutToWait, target);
    }

    pub fn window_target(&self) -> &event_loop::ActiveEventLoop {
        &self.window_target
    }

    pub fn create_proxy(&self) -> EventLoopProxy<T> {
        EventLoopProxy {
            user_events_sender: self.user_events_sender.clone(),
            waker: self.window_target.p.waker.clone(),
        }
    }
}

pub struct EventLoopProxy<T: 'static> {
    user_events_sender: mpsc::Sender<T>,
    waker: Arc<toyos_window::Waker>,
}

impl<T> EventLoopProxy<T> {
    pub fn send_event(&self, event: T) -> Result<(), event_loop::EventLoopClosed<T>> {
        self.user_events_sender
            .send(event)
            .map_err(|mpsc::SendError(x)| event_loop::EventLoopClosed(x))?;
        self.waker.wake();
        Ok(())
    }
}

impl<T> Clone for EventLoopProxy<T> {
    fn clone(&self) -> Self {
        Self { user_events_sender: self.user_events_sender.clone(), waker: self.waker.clone() }
    }
}

impl<T> Unpin for EventLoopProxy<T> {}

pub struct ActiveEventLoop {
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
    /// Ends the loop's wait. A window is `Send`, so it may be redrawn or dropped
    /// from a thread other than `loop_thread`, and such a push is woken.
    pub(super) waker: Arc<toyos_window::Waker>,
    pub(super) loop_thread: ThreadId,
    pub(super) creates: Mutex<VecDeque<(Arc<Mutex<toyos_window::Window>>, WindowId)>>,
    pub(super) redraws: Arc<Mutex<Redraws>>,
    pub(super) destroys: Arc<Mutex<VecDeque<WindowId>>>,
}

impl ActiveEventLoop {
    pub fn create_custom_cursor(&self, source: CustomCursorSource) -> RootCustomCursor {
        let _ = source.inner;
        RootCustomCursor { inner: super::PlatformCustomCursor }
    }

    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        None
    }

    pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
        VecDeque::new()
    }

    #[inline]
    pub fn listen_device_events(&self, _allowed: DeviceEvents) {}

    #[inline]
    pub fn system_theme(&self) -> Option<Theme> {
        None
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::RawDisplayHandle::ToyOs(rwh_06::ToyOsDisplayHandle::new()))
    }

    pub fn set_control_flow(&self, control_flow: ControlFlow) {
        self.control_flow.set(control_flow)
    }

    pub fn control_flow(&self) -> ControlFlow {
        self.control_flow.get()
    }

    pub(crate) fn exit(&self) {
        self.exit.set(true);
    }

    pub(crate) fn clear_exit(&self) {
        self.exit.set(false)
    }

    pub(crate) fn exiting(&self) -> bool {
        self.exit.get()
    }

    pub(crate) fn owned_display_handle(&self) -> OwnedDisplayHandle {
        OwnedDisplayHandle
    }
}

#[derive(Clone)]
pub(crate) struct OwnedDisplayHandle;

impl OwnedDisplayHandle {
    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::RawDisplayHandle::ToyOs(rwh_06::ToyOsDisplayHandle::new()))
    }
}
