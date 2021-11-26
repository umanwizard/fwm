use fwm::ChildLocation;

use ::fwm::AreaSize;
use ::fwm::Direction;
use ::fwm::ItemIdx;
use ::fwm::Layout;
use ::fwm::LayoutAction;
use ::fwm::MoveCursor;
use ::fwm::Position;
use ::fwm::WindowBounds;
use fwm::ItemAndData;
use fwm::LayoutDataMut;
use fwm::LayoutDataRef;

use fwm::scheme::Deserializer;
use fwm::scheme::Serializer;
use rand::distributions::{Distribution, Standard};
use rand::thread_rng;
use rand::Rng;
use rust_guile::scm_apply_1;
use rust_guile::scm_apply_2;
use rust_guile::scm_apply_3;
use rust_guile::scm_assert_foreign_object_type;
use rust_guile::scm_assq_ref;
use rust_guile::scm_c_define_gsubr;
use rust_guile::scm_car_unchecked;
use rust_guile::scm_cdr_unchecked;
use rust_guile::scm_eq_p;
use rust_guile::scm_foreign_object_ref;
use rust_guile::scm_from_uint64;
use rust_guile::scm_from_utf8_stringn;
use rust_guile::scm_from_utf8_symbol;
use rust_guile::scm_gc_malloc;
use rust_guile::scm_gc_malloc_pointerless;
use rust_guile::scm_gc_protect_object;
use rust_guile::scm_gc_unprotect_object;
use rust_guile::scm_is_pair;
use rust_guile::scm_is_truthy;
use rust_guile::scm_list_1;
use rust_guile::scm_make_foreign_object_1;
use rust_guile::scm_make_foreign_object_type;
use rust_guile::scm_primitive_eval;
use rust_guile::scm_procedure_p;
use rust_guile::scm_shell;
use rust_guile::scm_to_uint64;
use rust_guile::scm_to_utf8_string;
use rust_guile::scm_to_utf8_stringn;
use rust_guile::scm_with_guile;
use rust_guile::scm_wrong_type_arg_msg;
use rust_guile::size_t;
use rust_guile::SCM;
use rust_guile::SCM_EOL;
use rust_guile::SCM_UNSPECIFIED;
use serde::Deserialize;
use serde::Serialize;
use x11::keysym::XK_4;
use x11::keysym::XK_F4;
use x11::xlib::ConfigureNotify;
use x11::xlib::ControlMask;
use x11::xlib::CurrentTime;
use x11::xlib::Display;
use x11::xlib::GrabModeAsync;
use x11::xlib::InputOnly;
use x11::xlib::KeyPress;
use x11::xlib::KeySym;
use x11::xlib::LockMask;
use x11::xlib::Mod1Mask;
use x11::xlib::Mod2Mask;
use x11::xlib::Mod3Mask;
use x11::xlib::Mod4Mask;
use x11::xlib::Mod5Mask;
use x11::xlib::NoSymbol;
use x11::xlib::PointerRoot;
use x11::xlib::RevertToNone;
use x11::xlib::RevertToPointerRoot;
use x11::xlib::ShiftMask;
use x11::xlib::StructureNotifyMask;
use x11::xlib::SubstructureNotifyMask;
use x11::xlib::SubstructureRedirectMask;
use x11::xlib::XConfigureEvent;
use x11::xlib::XConfigureRequestEvent;
use x11::xlib::XConfigureWindow;
use x11::xlib::XCreateSimpleWindow;
use x11::xlib::XCreateWindowEvent;
use x11::xlib::XDefaultRootWindow;
use x11::xlib::XDestroyWindow;
use x11::xlib::XDestroyWindowEvent;
use x11::xlib::XErrorEvent;
use x11::xlib::XEvent;
use x11::xlib::XGetWindowAttributes;
use x11::xlib::XGrabKey;
use x11::xlib::XGrabPointer;
use x11::xlib::XGrabServer;
use x11::xlib::XKeyEvent;
use x11::xlib::XKeyPressedEvent;
use x11::xlib::XKeycodeToKeysym;
use x11::xlib::XKeysymToKeycode;
use x11::xlib::XKeysymToString;
use x11::xlib::XKillClient;
use x11::xlib::XMapRequestEvent;
use x11::xlib::XMapWindow;
use x11::xlib::XMoveResizeWindow;
use x11::xlib::XNextEvent;
use x11::xlib::XOpenDisplay;
use x11::xlib::XRaiseWindow;
use x11::xlib::XReparentEvent;
use x11::xlib::XReparentWindow;
use x11::xlib::XResizeWindow;
use x11::xlib::XScreenCount;
use x11::xlib::XScreenOfDisplay;
use x11::xlib::XSelectInput;
use x11::xlib::XSendEvent;
use x11::xlib::XSetErrorHandler;
use x11::xlib::XSetIOErrorHandler;
use x11::xlib::XSetInputFocus;
use x11::xlib::XSetWindowAttributes;
use x11::xlib::XSetWindowBackground;
use x11::xlib::XSetWindowBorder;
use x11::xlib::XStringToKeysym;
use x11::xlib::XSync;
use x11::xlib::XUngrabKey;
use x11::xlib::XWindowAttributes;
use x11::xlib::XWindowChanges;

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryInto;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt::Debug;
use std::iter::empty;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::ptr::null;
use std::ptr::null_mut;
use std::rc::Rc;

// fn assert_send<T>()
// where
//     T: Send,
// {
// }

#[derive(Debug)]
struct ProtectedScm(SCM);

impl ProtectedScm {
    pub unsafe fn new(x: SCM) -> Self {
        scm_gc_protect_object(x);
        Self(x)
    }
}

impl Drop for ProtectedScm {
    fn drop(&mut self) {
        unsafe {
            scm_gc_unprotect_object(self.0);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct X11ClientWindowData {
    window: x11::xlib::Window,
    mapped: bool,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl From<Rgb> for u32 {
    fn from(x: Rgb) -> Self {
        ((x.r as u32) << 16) | ((x.g as u32) << 8) | (x.b as u32)
    }
}

impl From<Rgb> for u64 {
    fn from(x: Rgb) -> Self {
        let x: u32 = x.into();
        x as u64
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
struct WindowDecorationTemplate {
    color: Rgb,
    width: usize,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
struct WindowDecorationsTemplate {
    left: WindowDecorationTemplate,
    up: WindowDecorationTemplate,
    down: WindowDecorationTemplate,
    right: WindowDecorationTemplate,
}

impl WindowDecorationsTemplate {
    pub const fn from_one(one: &WindowDecorationTemplate) -> Self {
        Self {
            left: *one,
            up: *one,
            down: *one,
            right: *one,
        }
    }
}

const BASIC_DECO: WindowDecorationsTemplate =
    WindowDecorationsTemplate::from_one(&WindowDecorationTemplate {
        color: Rgb {
            r: 0,
            g: 0,
            b: 0xFF,
        },
        width: 3,
    });
const POINT_DECO: WindowDecorationsTemplate =
    WindowDecorationsTemplate::from_one(&WindowDecorationTemplate {
        color: Rgb {
            r: 0,
            g: 0xFF,
            b: 0,
        },
        width: 3,
    });

#[derive(Serialize, Deserialize, Debug)]
struct WindowDecorations {
    left: x11::xlib::Window,
    up: x11::xlib::Window,
    down: x11::xlib::Window,
    right: x11::xlib::Window,
}

unsafe fn make_decorations_for_frame(
    display: *mut Display,
    frame: x11::xlib::Window,
) -> WindowDecorations {
    let left = XCreateSimpleWindow(display, frame, 0, 0, 1, 1, 0, 0, 0);
    let up = XCreateSimpleWindow(display, frame, 0, 0, 1, 1, 0, 0, 0);
    let right = XCreateSimpleWindow(display, frame, 0, 0, 1, 1, 0, 0, 0);
    let down = XCreateSimpleWindow(display, frame, 0, 0, 1, 1, 0, 0, 0);
    XMapWindow(display, left);
    XMapWindow(display, up);
    XMapWindow(display, right);
    XMapWindow(display, down);
    WindowDecorations {
        left,
        up,
        down,
        right,
    }
}

unsafe fn configure_decorations(
    display: *mut Display,
    frame_size: AreaSize,
    d: &WindowDecorations,
    t: &WindowDecorationsTemplate,
) {
    XMoveResizeWindow(
        display,
        d.left,
        0,
        0,
        t.left.width.try_into().unwrap(),
        frame_size.height.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.up,
        0,
        0,
        frame_size.width.try_into().unwrap(),
        t.up.width.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.down,
        0,
        (frame_size.height - t.down.width).try_into().unwrap(),
        frame_size.width.try_into().unwrap(),
        t.down.width.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.right,
        (frame_size.width - t.right.width).try_into().unwrap(),
        0,
        t.down.width.try_into().unwrap(),
        frame_size.height.try_into().unwrap(),
    );

    XSetWindowBackground(display, d.left, t.left.color.into());
    XSetWindowBackground(display, d.up, t.up.color.into());
    XSetWindowBackground(display, d.down, t.down.color.into());
    XSetWindowBackground(display, d.right, t.right.color.into());
}

#[derive(Serialize, Deserialize, Debug)]
struct WindowData {
    // This is optional, to allow holes in the layout
    client: Option<X11ClientWindowData>,
    frame: x11::xlib::Window,
    decorations: WindowDecorations,
    template: WindowDecorationsTemplate,
    inner_size: AreaSize,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct ContainerData {
    inner_size: AreaSize,
}

#[derive(Debug)]
struct WmState {
    pub client_window_to_item_idx: HashMap<x11::xlib::Window, usize>,
    pub bindings: HashMap<KeyCombo, ProtectedScm>,
    //    pub on_point_changed: ProtectedScm,
    pub layout: Layout<WindowData, ContainerData>,
    pub point: ItemIdx,
    pub cursor: Option<MoveCursor>,

    pub display: *mut x11::xlib::Display,
    pub root: x11::xlib::Window,
    pub focused: Option<usize>,
    pub on_point_changed: ProtectedScm,
}

unsafe impl Send for WmState {}

const BG_COLOR: u64 = 0x00FF00FF;

fn outer_to_inner_size(outer: AreaSize, dt: &WindowDecorationsTemplate) -> AreaSize {
    AreaSize {
        width: outer.width.saturating_sub(dt.left.width + dt.right.width),
        height: outer.height.saturating_sub(dt.up.width + dt.down.width),
    }
}

impl WmState {
    unsafe fn call_on_point_changed(&mut self) {
        let point = self.point.serialize(Serializer::default()).expect("XXX");
        let on_point_changed = self.on_point_changed.0;
        let scm = make_foreign_object_from_ref(self, WM_STATE_TYPE);
        scm_apply_2(on_point_changed, scm.inner, point, SCM_EOL);
    }
    unsafe fn ensure_focus(&mut self) {
        let mut did = false;
        if let Some(focused) = self.focused {
            if let WindowData {
                client:
                    Some(X11ClientWindowData {
                        window,
                        mapped: true,
                    }),
                ..
            } = self
                .layout
                .try_window_data(focused)
                .expect("WmState::focused should have been cleared when the slot was destroyed.")
            {
                XSetInputFocus(self.display, *window, RevertToPointerRoot, CurrentTime);
                did = true;
            }
        }
        if !did {
            // XXX - not totally sure whether pointer root is correct here.
            XSetInputFocus(
                self.display,
                PointerRoot.try_into().unwrap(),
                RevertToPointerRoot,
                CurrentTime,
            );
        }
    }
    unsafe fn make_frame(&mut self) -> x11::xlib::Window {
        let frame = XCreateSimpleWindow(self.display, self.root, 0, 0, 1, 1, 0, 0, BG_COLOR);
        XSelectInput(
            self.display,
            frame,
            SubstructureRedirectMask | SubstructureNotifyMask,
        );

        XMapWindow(self.display, frame);
        frame
    }

    unsafe fn kill_window(&mut self, window: x11::xlib::Window) {
        // TODO - gracefully kill the window - we will need to design a protocol
        // to communicate with the layout about _attempting_ to kill windows.
        // For now we just nuke it.

        XDestroyWindow(self.display, window);
    }

    unsafe fn update_window_bounds(&mut self, window_idx: usize) {
        let WindowData {
            client,
            frame,
            inner_size,
            template,
            decorations,
        } = self
            .layout
            .try_data(ItemIdx::Window(window_idx))
            .expect("Client should exist here")
            .unwrap_window();
        let window = client.expect("Window should exist here").window;
        println!(
            "Resizing client window {:#x} to inner size {:?}",
            window, inner_size
        );
        XMoveResizeWindow(
            self.display,
            window,
            template.left.width.try_into().unwrap(),
            template.up.width.try_into().unwrap(),
            inner_size.width.try_into().unwrap(),
            inner_size.height.try_into().unwrap(),
        );
        let frame_bounds = self.layout.bounds(ItemIdx::Window(window_idx));
        println!(
            "Moving and resizing frame window {:#x} to bounds {:?}",
            frame, frame_bounds
        );
        XMoveResizeWindow(
            self.display,
            *frame,
            frame_bounds.position.x.try_into().unwrap(),
            frame_bounds.position.y.try_into().unwrap(),
            frame_bounds.content.width.try_into().unwrap(),
            frame_bounds.content.height.try_into().unwrap(),
        );
        configure_decorations(self.display, frame_bounds.content, &decorations, &template);
    }

    unsafe fn update_cursor(
        &mut self,
        old_cursor: Option<MoveCursor>,
        new_cursor: Option<MoveCursor>,
    ) {
        // TODO - cursor painting
    }

    unsafe fn update_point(&mut self, old_point: ItemIdx, new_point: ItemIdx) {
        println!("Updating point: {:?} to {:?}", old_point, new_point);

        if let ItemIdx::Window(old_w_idx) = old_point {
            let bounds = self.layout.bounds(old_point);
            if let Some(data) = self.layout.try_window_data_mut(old_w_idx) {
                println!("Data template is {:?}, setting to BASIC_DECO", data.template);
                if data.template != BASIC_DECO {
                    data.template = BASIC_DECO;
                    configure_decorations(
                        self.display,
                        bounds.content,
                        &data.decorations,
                        &data.template,
                    );
                }
            }
        }

        if let ItemIdx::Window(new_w_idx) = new_point {
            let bounds = self.layout.bounds(new_point);
            let data = self.layout.try_window_data_mut(new_w_idx).unwrap();
            println!("Data template is {:?}, setting to POINT_DECO", data.template);
            if data.template != POINT_DECO {
                data.template = POINT_DECO;
                configure_decorations(
                    self.display,
                    bounds.content,
                    &data.decorations,
                    &data.template,
                );
            }
        }

        if old_point != new_point {
            self.call_on_point_changed();
        }
    }

    pub fn get_frame(&self, w_idx: usize) -> x11::xlib::Window {
        self.layout.try_window_data(w_idx).unwrap().frame
    }
    pub fn set_frame(&mut self, w_idx: usize, frame: x11::xlib::Window) {
        let pd = self.layout.try_window_data_mut(w_idx).unwrap();
        pd.frame = frame;
    }
}

impl WmState {
    pub fn new<'a>(
        display: *mut x11::xlib::Display,
        root: x11::xlib::Window,
        bounds: WindowBounds,
        on_point_changed: ProtectedScm,
        // uhh...
        frames_created: &'a mut HashSet<x11::xlib::Window>,
    ) -> Self {
        let mut ret = Self {
            client_window_to_item_idx: Default::default(),
            bindings: Default::default(),
            on_point_changed,
            layout: Layout::new_in_bounds(bounds),
            point: ItemIdx::Container(0),
            cursor: None,
            focused: None,

            display,
            root,
        };
        unsafe {
            // let frame = ret.make_frame(ItemIdx::Container(0));
            // frames_created.insert(frame);
        }
        ret
    }
    pub fn do_and_recompute<I, F>(&mut self, closure: F)
    where
        I: IntoIterator<Item = LayoutAction<WindowData, ContainerData>>,
        F: FnOnce(&mut Self) -> I,
    {
        let old_point = self.point;
        let old_cursor = self.cursor;
        let actions = closure(self);
        let new_point = self.point;
        let new_cursor = self.cursor;

        for action in actions {
            eprintln!("Running action: {:?}", action);
            self.update_for_action(action);
        }
        unsafe {
            self.update_point(old_point, new_point);
        }
        unsafe {
            self.update_cursor(old_cursor, new_cursor);
        }
    }
    pub fn update_for_action(&mut self, action: LayoutAction<WindowData, ContainerData>) {
        match action {
            LayoutAction::NewBounds { idx, bounds } => {
                let inner_size = match idx {
                    ItemIdx::Window(w_idx) => {
                        let data = self.layout.try_window_data(w_idx).unwrap();
                        outer_to_inner_size(bounds.content, &data.template)
                    }
                    ItemIdx::Container(c_idx) => bounds.content,
                };
                self.set_inner_size(idx, inner_size);
                if let ItemIdx::Window(w_idx) = idx {
                    if self
                        .layout
                        .try_data(idx)
                        .map(|md| md.unwrap_window().client.is_some())
                        .unwrap_or(false)
                    {
                        unsafe {
                            self.update_window_bounds(w_idx);
                        }
                    }
                }
            }
            LayoutAction::ItemDestroyed { item } => {
                if let ItemAndData::Window(idx, data) = &item {
                    if self.focused == Some(*idx) {
                        self.focused = None;
                    }
                    if let Some(window) = data.client {
                        unsafe {
                            self.kill_window(window.window);
                        }
                    }
                }
                match item {
                    ItemAndData::Window(_, data) => unsafe { self.kill_window(data.frame) },
                    ItemAndData::Container(_, _) => {}
                };
            }
            LayoutAction::ItemHidden { idx: _ } => unimplemented!(),
        }
    }
    pub fn navigate(&mut self, direction: Direction) {
        self.do_and_recompute(|wm| {
            if let Some(point) = wm.layout.navigate(wm.point, direction, None) {
                wm.point = point;
            }
            None
        });
    }
    pub fn navigate_cursor(&mut self, direction: Direction) {
        self.do_and_recompute(|wm| {
            let cursor = wm
                .cursor
                .unwrap_or_else(|| wm.layout.cursor_before(wm.point));
            let new_cursor = match cursor {
                MoveCursor::Split {
                    item,
                    direction: split_direction,
                } => wm
                    .layout
                    .navigate(item, direction, None)
                    .map(|new_item| MoveCursor::Split {
                        item: new_item,
                        direction: split_direction,
                    })
                    .unwrap_or(cursor),
                MoveCursor::Into { container, index } => wm
                    .layout
                    .navigate2(
                        ChildLocation { container, index },
                        direction,
                        None,
                        true,
                        false,
                    )
                    .map(|ChildLocation { container, index }| MoveCursor::Into { container, index })
                    .unwrap_or(cursor),
            };
            wm.cursor = (new_cursor != wm.layout.cursor_before(wm.point)).then(|| new_cursor);
            None
        });
    }
    fn get_inner_size(&self, idx: ItemIdx) -> AreaSize {
        match self.layout.try_data(idx).unwrap() {
            fwm::LayoutDataRef::Window(data) => data.inner_size,
            fwm::LayoutDataRef::Container(data) => data.inner_size,
        }
    }
    fn set_inner_size(&mut self, idx: ItemIdx, inner_size: AreaSize) {
        match self.layout.try_data_mut(idx).unwrap() {
            fwm::LayoutDataMut::Window(data) => data.inner_size = inner_size,
            fwm::LayoutDataMut::Container(data) => data.inner_size = inner_size,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
struct KeyCombo {
    key_sym: KeySym,
    shift: bool,
    lock: bool,
    control: bool,
    mod1: bool,
    mod2: bool,
    mod3: bool,
    mod4: bool,
    mod5: bool,
}

impl KeyCombo {
    pub fn x_modifiers(&self) -> u32 {
        self.shift.then(|| ShiftMask).unwrap_or(0)
            | self.lock.then(|| LockMask).unwrap_or(0)
            | self.control.then(|| ControlMask).unwrap_or(0)
            | self.mod1.then(|| Mod1Mask).unwrap_or(0)
            | self.mod2.then(|| Mod2Mask).unwrap_or(0)
            | self.mod3.then(|| Mod3Mask).unwrap_or(0)
            | self.mod4.then(|| Mod4Mask).unwrap_or(0)
            | self.mod5.then(|| Mod5Mask).unwrap_or(0)
    }
    pub fn from_x(key_sym: KeySym, state: u32) -> Self {
        let shift = (state & ShiftMask) != 0;
        let lock = (state & LockMask) != 0;
        let control = (state & ControlMask) != 0;
        let mod1 = (state & Mod1Mask) != 0;
        let mod2 = (state & Mod2Mask) != 0;
        let mod3 = (state & Mod3Mask) != 0;
        let mod4 = (state & Mod4Mask) != 0;
        let mod5 = (state & Mod5Mask) != 0;

        Self {
            key_sym,
            shift,
            lock,
            control,
            mod1,
            mod2,
            mod3,
            mod4,
            mod5,
        }
    }
}

impl std::fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.shift {
            write!(f, "shift+")?;
        }
        if self.lock {
            write!(f, "lock+")?;
        }
        if self.control {
            write!(f, "control+")?;
        }
        if self.mod1 {
            write!(f, "mod1+")?;
        }
        if self.mod2 {
            write!(f, "mod2+")?;
        }
        if self.mod3 {
            write!(f, "mod3+")?;
        }
        if self.mod4 {
            write!(f, "mod4+")?;
        }
        if self.mod5 {
            write!(f, "mod5+")?;
        }
        let s = unsafe { CStr::from_ptr(XKeysymToString(self.key_sym as u64)) }
            .to_str()
            .unwrap();
        write!(f, "{}", s)?;
        Ok(())
    }
}

static mut KEY_COMBO_TYPE: SCM = SCM_UNSPECIFIED;
static mut WM_STATE_TYPE: SCM = SCM_UNSPECIFIED;

unsafe extern "C" fn write_key_combo(kc: SCM) -> SCM {
    let s = format!("{}", get_foreign_object::<KeyCombo>(KEY_COMBO_TYPE, kc));
    scm_from_utf8_stringn(std::mem::transmute(s.as_ptr()), s.len() as u64)
}

unsafe extern "C" fn parse_key_combo(code_string: SCM) -> SCM {
    let mut len: size_t = 0;
    let s = scm_to_utf8_stringn(code_string, &mut len) as *mut u8;
    let len = len.try_into().unwrap();
    let s = String::from_raw_parts(s, len, len);

    let mut ks = None;
    let mut shift = false;
    let mut lock = false;
    let mut control = false;
    let mut mod1 = false;
    let mut mod2 = false;
    let mut mod3 = false;
    let mut mod4 = false;
    let mut mod5 = false;

    for part in s.split("+") {
        match part {
            "shift" => shift = true,
            "lock" => lock = true,
            "control" => control = true,
            "mod1" => mod1 = true,
            "mod2" => mod2 = true,
            "mod3" => mod3 = true,
            "mod4" => mod4 = true,
            "mod5" => mod5 = true,
            part => {
                let part = CString::new(part).expect("XXX: Return error to scheme");
                match XStringToKeysym(part.as_ptr()) {
                    0 => panic!("XXX"),
                    sym => ks = Some(sym),
                }
            }
        }
    }

    let ks = ks.expect("XXX");
    let combo = KeyCombo {
        key_sym: ks,
        shift,
        lock,
        control,
        mod1,
        mod2,
        mod3,
        mod4,
        mod5,
    };

    make_foreign_object(combo, b"KeyCombo\0", KEY_COMBO_TYPE)
}

struct LifetimeScm<'a> {
    inner: SCM,
    marker: PhantomData<&'a mut ()>,
}

unsafe fn make_foreign_object_from_ref<'a, T: Send>(
    obj: &'a mut T,
    r#type: SCM,
) -> LifetimeScm<'a> {
    let inner = scm_make_foreign_object_1(r#type, (obj as *mut T) as *mut c_void);
    LifetimeScm {
        inner,
        marker: Default::default(),
    }
}

unsafe fn make_foreign_object<T: Send>(obj: T, name: &[u8], r#type: SCM) -> SCM {
    let storage = scm_gc_malloc_pointerless(
        size_of::<T>() as u64,
        CStr::from_bytes_with_nul(name).unwrap().as_ptr(),
    ) as *mut T;
    std::ptr::write(storage, obj);
    scm_make_foreign_object_1(r#type, storage as *mut c_void)
}

unsafe fn get_foreign_object<'a, T>(obj: SCM, r#type: SCM) -> &'a mut T {
    scm_assert_foreign_object_type(r#type, obj);
    let p = scm_foreign_object_ref(obj, 0) as *mut T;
    p.as_mut()
        .expect("We should have set data in the constructor")
}

unsafe extern "C" fn x_err(_display: *mut Display, ev: *mut XErrorEvent) -> i32 {
    println!("X error: {:?}", *ev);
    0
}

unsafe extern "C" fn x_io_err(_display: *mut Display) -> i32 {
    let e = std::io::Error::last_os_error();
    println!("X io error (last: {:?})", e);
    0
}

unsafe extern "C" fn run_wm(config: SCM) -> SCM {
    let bindings = scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"bindings\0")),
    );
    let place_new_window = scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"place-new-window\0")),
    );
    let on_point_changed = ProtectedScm(scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"on-point-changed\0")),
    ));
    let display = XOpenDisplay(null());
    assert!(!display.is_null());
    XSetErrorHandler(Some(x_err));
    XSetIOErrorHandler(Some(x_io_err));
    let root = XDefaultRootWindow(display);
    XSelectInput(
        display,
        root,
        SubstructureRedirectMask | SubstructureNotifyMask,
    );
    XSync(display, 0);

    let n_screens = XScreenCount(display);
    assert!(n_screens > 0);
    let screen = XScreenOfDisplay(display, 0);
    let screen = std::ptr::read(screen);

    // These should not themselves be framed.
    let mut frames_created = HashSet::new();

    let wm = WmState::new(
        display,
        root,
        WindowBounds {
            position: Default::default(),
            content: AreaSize {
                width: screen.width.try_into().unwrap(),
                height: screen.height.try_into().unwrap(),
            },
        },
        on_point_changed,
        &mut frames_created,
    );

    let wm_scm = make_foreign_object(wm, b"WmState\0", WM_STATE_TYPE);

    insert_bindings(wm_scm, bindings);

    // XGrabServer(display);
    // ... rehome windows ...
    // XUngrabServer(display);

    loop {
        let mut e = MaybeUninit::<XEvent>::uninit();
        XNextEvent(display, e.as_mut_ptr());
        let e = e.assume_init();
        println!("Event: {:?}", e);
        match e.type_ {
            x11::xlib::KeyPress => {
                let XKeyEvent { keycode, state, .. } = e.key;
                let keysym = XKeycodeToKeysym(display, keycode.try_into().unwrap(), 0); // TODO - figure out what the zero means here.

                let combo = KeyCombo::from_x(keysym, state);
                println!("{}", combo);
                let proc = {
                    let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                    wm.bindings[&combo].0 // XXX
                };
                scm_apply_1(proc, wm_scm, SCM_EOL);
            }
            x11::xlib::CreateNotify => {
                let XCreateWindowEvent { window, .. } = e.create_window;
                {}
            }
            x11::xlib::ConfigureRequest => {
                // Let windows do whatever they want if we haven't taken them over yet.
                let ev = e.configure_request;
                let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                match wm.client_window_to_item_idx.get(&ev.window).copied() {
                    None => {
                        let mut changes = XWindowChanges {
                            x: ev.x,
                            y: ev.y,
                            width: ev.width,
                            height: ev.height,
                            border_width: ev.border_width,
                            sibling: ev.above, // no clue, but this is what basic_wm does.
                            stack_mode: ev.detail, // idem
                        };
                        XConfigureWindow(
                            display,
                            ev.window,
                            ev.value_mask.try_into().unwrap(),
                            &mut changes as *mut XWindowChanges,
                        );
                    }
                    Some(w_idx) => {
                        // We already control you -- sorry, but you don't get to fight with us about position/size.
                        // Notify you of your real coordinates.
                        let data = wm.layout.try_window_data(w_idx).unwrap();
                        let idx = ItemIdx::Window(w_idx);
                        let WindowBounds {
                            content: _,
                            position,
                        } = wm.layout.bounds(idx);

                        let ev2 = XConfigureEvent {
                            type_: ConfigureNotify,
                            serial: 0,
                            send_event: 1,
                            display,
                            event: ev.window,
                            window: ev.window,
                            x: (position.x + data.template.left.width).try_into().unwrap(),
                            y: (position.y + data.template.up.width).try_into().unwrap(),
                            width: data.inner_size.width.try_into().unwrap(),
                            height: data.inner_size.height.try_into().unwrap(),
                            border_width: 0,
                            above: 0,
                            override_redirect: 0, // ??? XXX
                        };
                        let mut ev2 = XEvent { configure: ev2 };

                        let status = XSendEvent(
                            display,
                            ev.window,
                            1,
                            StructureNotifyMask,
                            &mut ev2 as *mut XEvent,
                        );
                        println!("XSendEvent status for synthetic configure: {}", status);
                    }
                }
            }

            x11::xlib::MapRequest => {
                let XMapRequestEvent { window, .. } = e.map_request;
                if frames_created.contains(&window) || window == root {
                    // This prevents creating nested frames in an infinite loop.
                    continue;
                }
                // let mut attributes = MaybeUninit::<XWindowAttributes>::uninit();
                // XGetWindowAttributes(display, window, attributes.as_mut_ptr());
                // let attributes = attributes.assume_init();
                // if attributes.class == InputOnly {
                //     println!("InputOnly window -- not inserting it.");
                //     continue;
                // }

                let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                let already_mapped = wm.client_window_to_item_idx.contains_key(&window);

                if !already_mapped {
                    let insert_cursor = scm_apply_1(place_new_window, wm_scm, SCM_EOL);
                    let insert_cursor =
                        MoveOrReplace::deserialize(Deserializer { scm: insert_cursor })
                            .expect("XXX");
                    wm.do_and_recompute(|wm| match insert_cursor {
                        MoveOrReplace::Move(insert_cursor) => {
                            let frame = wm.make_frame();
                            let decorations = make_decorations_for_frame(display, frame);
                            let w_idx = wm.layout.alloc_window(WindowData {
                                client: Some(X11ClientWindowData {
                                    window,
                                    mapped: false,
                                }),
                                frame,
                                inner_size: Default::default(),
                                decorations,
                                template: BASIC_DECO,
                            });
                            wm.client_window_to_item_idx.insert(window, w_idx);
                            let actions = wm.layout.r#move(ItemIdx::Window(w_idx), insert_cursor);
                            wm.point = ItemIdx::Window(w_idx);
                            frames_created.insert(frame);
                            println!("Reparenting {:#x} into {:#x}", window, frame);
                            XReparentWindow(wm.display, window, frame, 0, 0);
                            XRaiseWindow(wm.display, window);
                            actions
                        }
                        MoveOrReplace::Replace(ItemIdx::Window(w_idx)) => {
                            let old_frame = wm.get_frame(w_idx);
                            let old_bounds = wm.layout.bounds(ItemIdx::Window(w_idx));

                            wm.client_window_to_item_idx.insert(window, w_idx);
                            wm.point = ItemIdx::Window(w_idx);
                            wm.set_frame(w_idx, 0);
                            wm.layout
                                .try_data_mut(wm.point)
                                .unwrap()
                                .unwrap_window()
                                .client = Some(X11ClientWindowData {
                                window,
                                mapped: false,
                            });
                            let frame = wm.make_frame();
                            frames_created.insert(frame);
                            println!("Reparenting {:#x} into {:#x}", window, frame);
                            XReparentWindow(wm.display, window, frame, 0, 0);
                            XRaiseWindow(wm.display, window);
                            XDestroyWindow(wm.display, old_frame);
                            frames_created.remove(&old_frame);
                            vec![LayoutAction::NewBounds {
                                idx: wm.point,
                                bounds: old_bounds,
                            }]
                        }
                        MoveOrReplace::Replace(ItemIdx::Container(_c_idx)) => todo!(),
                    });
                }
                XMapWindow(display, window);
            }
            x11::xlib::MapNotify => {
                let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                let ev = e.map;
                if let Some(&idx) = wm.client_window_to_item_idx.get(&ev.window) {
                    if let Some(WindowData { client, .. }) = wm.layout.try_window_data_mut(idx) {
                        let mut client = client
                            .as_mut()
                            .expect("Layout out of sync with client_window_to_item_idx");
                        client.mapped = true;
                    }
                }
                wm.ensure_focus();
            }
            x11::xlib::UnmapNotify => {
                let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                let ev = e.unmap;
                if let Some(&idx) = wm.client_window_to_item_idx.get(&ev.window) {
                    if let Some(WindowData {
                        client: Some(client),
                        ..
                    }) = wm.layout.try_window_data_mut(idx)
                    {
                        client.mapped = false;
                    }
                }
                wm.ensure_focus();
            }
            x11::xlib::DestroyNotify => {
                let XDestroyWindowEvent { window, .. } = e.destroy_window;
                let wm = get_foreign_object::<WmState>(wm_scm, WM_STATE_TYPE);
                if let Entry::Occupied(oe) = wm.client_window_to_item_idx.entry(window) {
                    let idx = oe.remove();
                    if wm.layout.exists(ItemIdx::Window(idx)) {
                        wm.layout
                            .try_data_mut(ItemIdx::Window(idx))
                            .unwrap()
                            .unwrap_window()
                            .client = None;
                    }
                }
                // TODO - call a scheme function to see whether the user wants to kill the
                // layout slot too
            }
            _ => {}
        }
    }
}

#[derive(Serialize, Deserialize)]
enum SpatialDir {
    Planar(Direction),
    Parent,
    Child,
}

unsafe extern "C" fn navigate(state: SCM, dir: SCM) -> SCM {
    let dir = match SpatialDir::deserialize(Deserializer { scm: dir }).expect("XXX") {
        SpatialDir::Planar(dir) => dir,
        _ => todo!(),
    };
    println!("dir is {:?}", dir);
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.navigate(dir);
    SCM_UNSPECIFIED
}

unsafe extern "C" fn cursor(state: SCM, dir: SCM) -> SCM {
    let dir = match SpatialDir::deserialize(Deserializer { scm: dir }).expect("XXX") {
        SpatialDir::Planar(dir) => dir,
        _ => todo!(),
    };
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.navigate_cursor(dir);
    SCM_UNSPECIFIED
}

unsafe fn scm_car(scm: SCM) -> SCM {
    if !scm_is_pair(scm) {
        scm_wrong_type_arg_msg(
            std::mem::transmute(b"car\0"),
            0,
            scm,
            std::mem::transmute(b"pair\0"),
        );
    }
    scm_car_unchecked(scm)
}

unsafe fn scm_cdr(scm: SCM) -> SCM {
    if !scm_is_pair(scm) {
        scm_wrong_type_arg_msg(
            std::mem::transmute(b"cdr\0"),
            0,
            scm,
            std::mem::transmute(b"pair\0"),
        );
    }
    scm_cdr_unchecked(scm)
}

unsafe extern "C" fn get_point(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.point.serialize(Serializer::default()).expect("XXX")
}

unsafe fn scm_from_bool(x: bool) -> SCM {
    if x {
        SCM_BOOL_T
    } else {
        SCM_BOOL_F
    }
}

unsafe extern "C" fn is_occupied(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    scm_from_bool(match point {
        ItemIdx::Container(_) => true, // Containers always count as occupied, since their frame is their entire content.
        ItemIdx::Window(w_idx) => wm
            .layout
            .try_window_data(w_idx)
            .expect("XXX")
            .client
            .is_some(),
    })
}

unsafe extern "C" fn nearest_container(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let ctr = wm.layout.nearest_container(point);
    scm_from_uint64(ctr as u64)
}

unsafe extern "C" fn n_children(state: SCM, ctr: SCM) -> SCM {
    let ctr = scm_to_uint64(ctr).try_into().unwrap();
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let n = wm.layout.n_children(ItemIdx::Container(ctr));
    scm_from_uint64(n as u64)
}

// unsafe fn cursor_to_scm(cursor: MoveCursor) -> SCM {
//     let (car, cdr) = match cursor {
//         MoveCursor::Split { item, direction } => {
//             let car = scm_from_utf8_symbol(std::mem::transmute(b"split\0"));
//             let scm_item = item_idx_to_scm(item);
//             let scm_direction = scm_from_utf8_symbol(match direction {
//                 Direction::Up => std::mem::transmute(b"up\0"),
//                 Direction::Down => std::mem::transmute(b"down\0"),
//                 Direction::Left => std::mem::transmute(b"left\0"),
//                 Direction::Right => std::mem::transmute(b"right\0"),
//             });
//             let cdr = scm_cons(scm_item, scm_direction);
//             (car, cdr)
//         }
//         MoveCursor::Into { container, index } => {
//             let car = scm_from_utf8_symbol(std::mem::transmute(b"into\0"));
//             let cdr = scm_cons(
//                 scm_from_uint64(container as u64),
//                 scm_from_uint64(index as u64),
//             );
//             (car, cdr)
//         }
//     };
//     scm_cons(car, cdr)
// }

// unsafe fn cursor_from_scm(scm: SCM) -> MoveCursor {
//     let (car, cdr) = (scm_car(scm), scm_cdr(scm));
//     if scm_is_eq(car, scm_from_utf8_symbol(std::mem::transmute(b"split\0"))) {
//         let (car, cdr) = (scm_car(cdr), scm_cdr(cdr));
//         let item = item_idx_from_scm(car);
//         let direction = if scm_is_eq(cdr, scm_from_utf8_symbol(std::mem::transmute(b"up\0"))) {
//             Direction::Up
//         } else if scm_is_eq(cdr, scm_from_utf8_symbol(std::mem::transmute(b"down\0"))) {
//             Direction::Down
//         } else if scm_is_eq(cdr, scm_from_utf8_symbol(std::mem::transmute(b"left\0"))) {
//             Direction::Left
//         } else if scm_is_eq(cdr, scm_from_utf8_symbol(std::mem::transmute(b"right\0"))) {
//             Direction::Right
//         } else {
//             panic!("XXX")
//         };
//         MoveCursor::Split { item, direction }
//     } else if scm_is_eq(car, scm_from_utf8_symbol(std::mem::transmute(b"into\0"))) {
//         let (car, cdr) = (scm_car(cdr), scm_cdr(cdr));
//         let container = scm_to_uint64(car).try_into().unwrap();
//         let index = scm_to_uint64(cdr).try_into().unwrap();
//         MoveCursor::Into { container, index }
//     } else {
//         panic!("XXX")
//     }
// }

#[derive(Deserialize, Serialize)]
enum MoveOrReplace {
    Move(MoveCursor),
    Replace(ItemIdx),
}

unsafe extern "C" fn make_cursor_into(container: SCM, index: SCM) -> SCM {
    let container = scm_to_uint64(container).try_into().unwrap();
    let index = scm_to_uint64(index).try_into().unwrap();
    let cursor = MoveOrReplace::Move(MoveCursor::Into { container, index });
    cursor.serialize(Serializer::default()).expect("XXX")
}

unsafe extern "C" fn make_cursor_before(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);

    let cursor = MoveOrReplace::Move(wm.layout.cursor_before(point));
    cursor.serialize(Serializer::default()).expect("XXX")
}

unsafe extern "C" fn kill_item_at(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    println!("Killing item at {:?}", point);
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.do_and_recompute(|wm| {
        let topo_next = wm.layout.topological_next(wm.point);
        let actions = wm.layout.destroy(point);
        if !wm.layout.exists(wm.point) {
            wm.point = topo_next.unwrap_or_else(|| wm.layout.topological_last());
        }
        actions
    });
    println!("Finished do and recompute in kill_item_at");
    SCM_UNSPECIFIED
}

unsafe extern "C" fn kill_client_at(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.do_and_recompute(|wm| {
        if let Some(LayoutDataMut::Window(WindowData { client, .. })) =
            wm.layout.try_data_mut(point)
        {
            if let Some(window) = client.take() {
                wm.kill_window(window.window);
            }
        }
        None
    });
    SCM_UNSPECIFIED
}

const SCM_BOOL_F: SCM = 0x4 as SCM;
const SCM_BOOL_T: SCM = 0x404 as SCM;

unsafe extern "C" fn insert_bindings(state: SCM, mut bindings: SCM) -> SCM {
    let state = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    while scm_is_pair(bindings) {
        let binding = scm_car_unchecked(bindings);
        bindings = scm_cdr_unchecked(bindings);

        // XXX handle error
        assert!(scm_is_pair(binding));
        let kc = scm_car_unchecked(binding);
        let proc = scm_cdr_unchecked(binding);

        // XXX handle error
        assert!(scm_is_truthy(scm_procedure_p(proc)));
        let kc = get_foreign_object::<KeyCombo>(kc, KEY_COMBO_TYPE).clone();
        state.bindings.insert(kc, ProtectedScm::new(proc));
        XGrabKey(
            state.display,
            XKeysymToKeycode(state.display, kc.key_sym.into()).into(),
            kc.x_modifiers(),
            state.root,
            0,
            GrabModeAsync,
            GrabModeAsync,
        );
    }
    SCM_UNSPECIFIED
}

unsafe extern "C" fn clear_bindings(state: SCM) -> SCM {
    let state = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    for (kc, _) in state.bindings.drain() {
        XUngrabKey(
            state.display,
            XKeysymToKeycode(state.display, kc.key_sym.into()).into(),
            kc.x_modifiers(),
            state.root,
        );
    }
    SCM_UNSPECIFIED
}

unsafe extern "C" fn drop_wm_state(state: SCM) {
    let p = scm_foreign_object_ref(state, 0) as *const WmState;
    let state = std::ptr::read(p);
    std::mem::drop(state);
}

unsafe extern "C" fn dump_layout(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let s = format!("{}", serde_json::to_string_pretty(&wm.layout).unwrap());
    scm_from_utf8_stringn(std::mem::transmute(s.as_ptr()), s.len() as u64)
}

unsafe extern "C" fn set_focus(state: SCM, point: SCM) -> SCM {
    let mut wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let maybe_window: Option<usize> =
        Deserialize::deserialize(Deserializer { scm: point }).expect("XXX");
    if let Some(window) = maybe_window {
        assert!(wm.layout.exists(ItemIdx::Window(window))); // XXX
    }
    wm.focused = maybe_window;
    wm.ensure_focus();
    SCM_UNSPECIFIED
}

// TODO - codegen this, as well as translating Scheme objects to Rust objects in the function bodies
// (similar to what we did in PyTorch)
unsafe extern "C" fn scheme_setup(_data: *mut c_void) -> *mut c_void {
    let kc_name = scm_from_utf8_symbol(CStr::from_bytes_with_nul(b"key-combo\0").unwrap().as_ptr());
    let kc_slots = scm_list_1(scm_from_utf8_symbol(
        CStr::from_bytes_with_nul(b"data\0").unwrap().as_ptr(),
    ));
    KEY_COMBO_TYPE = scm_make_foreign_object_type(kc_name, kc_slots, None);

    let wm_name = scm_from_utf8_symbol(CStr::from_bytes_with_nul(b"wm-state\0").unwrap().as_ptr());
    let wm_slots = scm_list_1(scm_from_utf8_symbol(
        CStr::from_bytes_with_nul(b"data\0").unwrap().as_ptr(),
    ));
    WM_STATE_TYPE = scm_make_foreign_object_type(wm_name, wm_slots, Some(drop_wm_state));

    let c = CStr::from_bytes_with_nul(b"fwm-run-wm\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, run_wm as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-parse-key-combo\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, parse_key_combo as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-write-key-combo\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, write_key_combo as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-navigate\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, navigate as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-cursor\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, cursor as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-get-point\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, get_point as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-occupied?\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, is_occupied as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-nearest-container\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, nearest_container as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-n-children\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, n_children as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-make-cursor-into\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, make_cursor_into as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-make-cursor-before\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, make_cursor_before as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-kill-item-at\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, kill_item_at as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-kill-client-at\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, kill_client_at as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-dump-layout\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, dump_layout as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-set-focus\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, set_focus as *mut c_void);

    std::ptr::null_mut()
}

fn main() {
    unsafe {
        scm_with_guile(Some(scheme_setup), null_mut());
        scm_shell(0, null_mut());
    }
}
