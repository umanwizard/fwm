use byteorder::NativeEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use differential_dataflow::lattice::Lattice;
use fwm::ChildLocation;

use ::fwm::AreaSize;
use ::fwm::Direction;
use ::fwm::ItemIdx;
use ::fwm::Layout;
use ::fwm::LayoutAction;
use ::fwm::MoveCursor;
use ::fwm::WindowBounds;
use fwm::Constructor;
use fwm::ItemAndData;
use fwm::LayoutDataMut;
use fwm::LayoutStrategy;
use fwm::Position;
use fwm::SlotInContainer;

use btv_scheme::Deserializer;
use btv_scheme::Serializer;
use libc::umask;
use log::error;
use log::info;
use mio::unix::SourceFd;
use mio::Events;
use mio::Interest;
use mio::Poll;
use mio::Token;
use rust_guile::scm_apply_1;
use rust_guile::scm_apply_2;
use rust_guile::scm_assert_foreign_object_type;
use rust_guile::scm_assq_ref;
use rust_guile::scm_c_define_gsubr;
use rust_guile::scm_car_unchecked;
use rust_guile::scm_cdr_unchecked;
use rust_guile::scm_foreign_object_ref;
use rust_guile::scm_from_uint64;
use rust_guile::scm_from_utf8_stringn;
use rust_guile::scm_from_utf8_symbol;
use rust_guile::scm_gc_malloc_pointerless;
use rust_guile::scm_gc_protect_object;
use rust_guile::scm_gc_unprotect_object;
use rust_guile::scm_is_pair;
use rust_guile::scm_is_truthy;
use rust_guile::scm_list_1;
use rust_guile::scm_make_foreign_object_1;
use rust_guile::scm_make_foreign_object_type;
use rust_guile::scm_procedure_p;
use rust_guile::scm_shell;
use rust_guile::scm_to_uint64;
use rust_guile::scm_to_utf8_stringn;
use rust_guile::scm_with_guile;
use rust_guile::size_t;
use rust_guile::SCM;
use rust_guile::SCM_EOL;
use rust_guile::SCM_UNSPECIFIED;
use serde::Deserialize;
use serde::Serialize;
use timely::progress::frontier::MutableAntichain;
use x11::xlib::AnyPropertyType;
use x11::xlib::Atom;
use x11::xlib::Button1;
use x11::xlib::ButtonPressMask;
use x11::xlib::ButtonReleaseMask;
use x11::xlib::CWBorderWidth;
use x11::xlib::CWHeight;
use x11::xlib::CWWidth;
use x11::xlib::ClientMessage;
use x11::xlib::ClientMessageData;
use x11::xlib::ConfigureNotify;
use x11::xlib::ControlMask;
use x11::xlib::CurrentTime;
use x11::xlib::Display;
use x11::xlib::GrabModeAsync;
use x11::xlib::GrabModeSync;
use x11::xlib::KeySym;
use x11::xlib::LockMask;
use x11::xlib::Mod1Mask;
use x11::xlib::Mod2Mask;
use x11::xlib::Mod3Mask;
use x11::xlib::Mod4Mask;
use x11::xlib::Mod5Mask;
use x11::xlib::PointerRoot;
use x11::xlib::ReplayPointer;
use x11::xlib::RevertToPointerRoot;
use x11::xlib::ShiftMask;
use x11::xlib::StructureNotifyMask;
use x11::xlib::SubstructureNotifyMask;
use x11::xlib::SubstructureRedirectMask;
use x11::xlib::Success;
use x11::xlib::Window;
use x11::xlib::XAllowEvents;
use x11::xlib::XButtonEvent;
use x11::xlib::XClearWindow;
use x11::xlib::XClientMessageEvent;
use x11::xlib::XConfigureEvent;
use x11::xlib::XConfigureWindow;
use x11::xlib::XConnectionNumber;
use x11::xlib::XCreateSimpleWindow;
use x11::xlib::XDefaultRootWindow;
use x11::xlib::XDestroyWindow;
use x11::xlib::XDestroyWindowEvent;
use x11::xlib::XErrorEvent;
use x11::xlib::XEvent;
use x11::xlib::XFree;
use x11::xlib::XGetAtomName;
use x11::xlib::XGetWMProtocols;
use x11::xlib::XGetWindowProperty;
use x11::xlib::XGrabButton;
use x11::xlib::XGrabKey;
use x11::xlib::XInternAtom;
use x11::xlib::XKeyEvent;
use x11::xlib::XKeycodeToKeysym;
use x11::xlib::XKeysymToKeycode;
use x11::xlib::XKeysymToString;
use x11::xlib::XMapRequestEvent;
use x11::xlib::XMapWindow;
use x11::xlib::XMoveResizeWindow;
use x11::xlib::XNextEvent;
use x11::xlib::XOpenDisplay;
use x11::xlib::XPending;
use x11::xlib::XRaiseWindow;
use x11::xlib::XScreenCount;
use x11::xlib::XScreenOfDisplay;
use x11::xlib::XSelectInput;
use x11::xlib::XSendEvent;
use x11::xlib::XSetErrorHandler;
use x11::xlib::XSetIOErrorHandler;
use x11::xlib::XSetInputFocus;
use x11::xlib::XSetWindowBackground;
use x11::xlib::XStringToKeysym;
use x11::xlib::XSync;
use x11::xlib::XUngrabKey;
use x11::xlib::XUnmapWindow;
use x11::xlib::XWindowChanges;
use x11::xlib::CWX;
use x11::xlib::CWY;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::os::raw::c_int;
use std::os::raw::c_uchar;
use std::os::raw::c_ulong;
use std::os::unix::io::RawFd;
use std::ptr::null;
use std::ptr::null_mut;

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

const BASIC_CTR_DECO: WindowDecorationsTemplate =
    WindowDecorationsTemplate::from_one(&WindowDecorationTemplate {
        color: Rgb {
            r: 0xCC,
            g: 0xCC,
            b: 0xCC,
        },
        width: 6,
    });
const POINT_CTR_DECO: WindowDecorationsTemplate =
    WindowDecorationsTemplate::from_one(&WindowDecorationTemplate {
        color: Rgb {
            r: 0xF4,
            g: 0x94,
            b: 0x01,
        },
        width: 6,
    });

fn compute_deco(
    item: ItemIdx,
    point: ItemIdx,
    cursor: Option<MoveCursor>,
    where_am_i: Option<SlotInContainer>,
) -> WindowDecorationsTemplate {
    let move_direction_and_split = cursor.and_then(|c| {
        where_am_i.and_then(|w| match c {
            MoveCursor::Split {
                item: c_item,
                direction,
            } if item == c_item => Some((direction, true)),
            MoveCursor::Split { .. } => None,
            MoveCursor::Into { container, index } if container == w.c_idx => {
                if index == w.index {
                    let dir = match w.parent_strat {
                        LayoutStrategy::Horizontal => Direction::Left,
                        LayoutStrategy::Vertical => Direction::Up,
                    };
                    Some((dir, false))
                } else if index == w.index + 1 {
                    let dir = match w.parent_strat {
                        LayoutStrategy::Horizontal => Direction::Right,
                        LayoutStrategy::Vertical => Direction::Down,
                    };
                    Some((dir, false))
                } else {
                    None
                }
            }
            MoveCursor::Into { .. } => None,
        })
    });

    let is_at_point = item == point;
    let mut deco = match (item, is_at_point) {
        (ItemIdx::Window(_), true) => POINT_DECO,
        (ItemIdx::Window(_), false) => BASIC_DECO,
        (ItemIdx::Container(_), true) => POINT_CTR_DECO,
        (ItemIdx::Container(_), false) => BASIC_CTR_DECO,
    };

    if let Some((move_direction, is_split)) = move_direction_and_split {
        let color = if is_split {
            Rgb {
                r: 0xFF,
                g: 0,
                b: 0,
            }
        } else {
            Rgb {
                r: 0xFF,
                g: 0,
                b: 0xFF,
            }
        };
        let dir_color_mut = match move_direction {
            Direction::Left => &mut deco.left.color,
            Direction::Right => &mut deco.right.color,
            Direction::Up => &mut deco.up.color,
            Direction::Down => &mut deco.down.color,
        };
        *dir_color_mut = color;
    }
    deco
}

#[derive(Serialize, Deserialize, Debug)]
struct WindowDecorations {
    left: x11::xlib::Window,
    up: x11::xlib::Window,
    down: x11::xlib::Window,
    right: x11::xlib::Window,
}

unsafe fn make_decorations(display: *mut Display, root: x11::xlib::Window) -> WindowDecorations {
    let left = XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0);
    let up = XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0);
    let right = XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0);
    let down = XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0);
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
    bounds: WindowBounds,
    d: &WindowDecorations,
    t: &WindowDecorationsTemplate,
) {
    XMoveResizeWindow(
        display,
        d.left,
        bounds.position.x.try_into().unwrap(),
        bounds.position.y.try_into().unwrap(),
        t.left.width.try_into().unwrap(),
        bounds.content.height.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.up,
        bounds.position.x.try_into().unwrap(),
        bounds.position.y.try_into().unwrap(),
        bounds.content.width.try_into().unwrap(),
        t.up.width.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.down,
        bounds.position.x.try_into().unwrap(),
        (bounds.position.y + bounds.content.height - t.down.width)
            .try_into()
            .unwrap(),
        bounds.content.width.try_into().unwrap(),
        t.down.width.try_into().unwrap(),
    );
    XMoveResizeWindow(
        display,
        d.right,
        (bounds.position.x + bounds.content.width - t.right.width)
            .try_into()
            .unwrap(),
        bounds.position.y.try_into().unwrap(),
        t.down.width.try_into().unwrap(),
        bounds.content.height.try_into().unwrap(),
    );

    XSetWindowBackground(display, d.left, t.left.color.into());
    XSetWindowBackground(display, d.up, t.up.color.into());
    XSetWindowBackground(display, d.down, t.down.color.into());
    XSetWindowBackground(display, d.right, t.right.color.into());
    XClearWindow(display, d.left);
    XClearWindow(display, d.up);
    XClearWindow(display, d.right);
    XClearWindow(display, d.down);
}

#[derive(Serialize, Deserialize, Debug)]
struct WindowData {
    // This is optional, to allow holes in the layout
    client: Option<X11ClientWindowData>,
    decorations: WindowDecorations,
    template: WindowDecorationsTemplate,
}

#[derive(Debug)]
struct ContainerDataConstructor {
    display: *mut Display,
    root: x11::xlib::Window,
}

impl Constructor for ContainerDataConstructor {
    type Item = ContainerData;

    fn construct(&mut self) -> Self::Item {
        let decorations = unsafe { make_decorations(self.display, self.root) };
        let template = BASIC_CTR_DECO;
        ContainerData {
            decorations,
            template,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ContainerData {
    decorations: WindowDecorations,
    template: WindowDecorationsTemplate,
}

#[derive(Debug)]
struct WmState {
    pub client_window_to_item_idx: HashMap<x11::xlib::Window, usize>,
    pub bindings: HashMap<KeyCombo, ProtectedScm>,
    //    pub on_point_changed: ProtectedScm,
    pub layout: Layout<WindowData, ContainerData, ContainerDataConstructor>,
    pub point: ItemIdx,
    pub cursor: Option<MoveCursor>,

    pub display: *mut x11::xlib::Display,
    pub root: x11::xlib::Window,
    pub focused: Option<usize>,
    pub on_point_changed: ProtectedScm,
    pub delete_window_atom: Atom,
    pub protocols_atom: Atom,
    pub struts: HashMap<Window, StrutPartial>,
    pub struts_frontier: MutableAntichain<StrutPartial>,
    pub current_strut: StrutPartial,
    pub root_size: AreaSize,
}

unsafe impl Send for WmState {}

fn outer_to_inner_size(outer: AreaSize, dt: &WindowDecorationsTemplate) -> AreaSize {
    AreaSize {
        width: outer.width.saturating_sub(dt.left.width + dt.right.width),
        height: outer.height.saturating_sub(dt.up.width + dt.down.width),
    }
}

impl WmState {
    fn do_resize(&mut self) {
        let content_width = self
            .root_size
            .width
            .saturating_sub((self.current_strut.left + self.current_strut.right) as usize);
        let content_height = self
            .root_size
            .height
            .saturating_sub((self.current_strut.top + self.current_strut.bottom) as usize);
        let root_ctr = self.layout.displayed_root();
        let new_bounds = WindowBounds {
            position: Position {
                x: self.current_strut.left as usize,
                y: self.current_strut.top as usize,
                root_ctr,
            },
            content: AreaSize {
                height: content_height,
                width: content_width,
            },
        };
        if new_bounds != self.layout.root_bounds(ItemIdx::Container(root_ctr)) {
            self.do_and_recompute(|wm| wm.layout.resize(new_bounds))
        }
    }
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

    unsafe fn kill_window(&mut self, window: x11::xlib::Window) {
        // TODO - gracefully kill the window - we will need to design a protocol
        // to communicate with the layout about _attempting_ to kill windows.
        // For now we just nuke it.

        XDestroyWindow(self.display, window);
    }

    unsafe fn update_window_bounds(&mut self, window_idx: usize) {
        let WindowData {
            client, template, ..
        } = self
            .layout
            .try_data(ItemIdx::Window(window_idx))
            .expect("Client should exist here")
            .unwrap_window();
        let window = client.expect("Window should exist here").window;
        let bounds = self.layout.bounds(ItemIdx::Window(window_idx));
        let inner_size = outer_to_inner_size(bounds.content, &template);
        // We use XConfigureWindow here, rather than just XMoveResizeWindow,
        // to allow us to set the border width back to 0 in case the client changed
        // it before mapping (XTerm does this, for example)
        let value_mask = CWX | CWY | CWWidth | CWHeight | CWBorderWidth;
        let mut changes = XWindowChanges {
            x: (bounds.position.x + template.left.width)
                .try_into()
                .unwrap(),
            y: (bounds.position.y + template.up.width).try_into().unwrap(),
            width: inner_size.width.try_into().unwrap(),
            height: inner_size.height.try_into().unwrap(),
            border_width: 0,
            // The rest are ignored due to the mask
            sibling: 0,
            stack_mode: 0,
        };
        XConfigureWindow(
            self.display,
            window,
            value_mask.into(),
            (&mut changes) as *mut XWindowChanges,
        );
    }

    unsafe fn update_point_and_cursor(
        &mut self,
        old_point: ItemIdx,
        new_point: ItemIdx,
        old_cursor: Option<MoveCursor>,
        new_cursor: Option<MoveCursor>,
    ) {
        info!(
            "Updating point: {:?} to {:?} and cursor: {:?} to {:?}",
            old_point, new_point, old_cursor, new_cursor
        );
        let mut possibly_affected = vec![old_point, new_point];
        for cur in &[old_cursor, new_cursor] {
            if let Some(cur) = cur {
                match cur {
                    MoveCursor::Split { item, .. } => possibly_affected.push(*item),
                    MoveCursor::Into { container, index } => {
                        if let Some(&(_weight, child)) =
                            self.layout.children(*container).get(*index)
                        {
                            possibly_affected.push(child);
                        }
                        if let Some(&(_weight, child)) = (*index > 0)
                            .then(|| Some(()))
                            .and_then(|_| self.layout.children(*container).get(*index - 1))
                        {
                            possibly_affected.push(child);
                        }
                    }
                }
            }
        }

        for item in possibly_affected {
            if self.layout.exists(item) {
                let where_is_it = self.layout.slot_in_container(item);
                let t = compute_deco(item, new_point, new_cursor, where_is_it);
                let bounds = self.layout.bounds(item);
                let mt = self.try_template_mut(item).unwrap();
                if *mt != t {
                    *mt = t;
                    let rt = self.try_template(item).unwrap();
                    configure_decorations(
                        self.display,
                        bounds,
                        self.try_decorations(item).unwrap(),
                        rt,
                    );
                }
            }
        }

        if old_point != new_point {
            self.call_on_point_changed();
        }
    }

    unsafe fn request_unmap(&mut self, window: x11::xlib::Window) {
        let ret = XUnmapWindow(self.display, window);
        if ret == 0 {
            error!("XUnmapWindow call failed for {window}");
        }
    }

    unsafe fn request_map(&mut self, window: x11::xlib::Window) {
        let ret = XMapWindow(self.display, window);
        if ret == 0 {
            error!("XMapWindow call failed for {window}");
        }
    }
}

impl WmState {
    pub fn new<'a>(
        display: *mut x11::xlib::Display,
        root: x11::xlib::Window,
        bounds: WindowBounds,
        on_point_changed: ProtectedScm,
    ) -> Self {
        let delete_window_atom =
            unsafe { XInternAtom(display, std::mem::transmute(b"WM_DELETE_WINDOW\0"), 0) };
        let protocols_atom =
            unsafe { XInternAtom(display, std::mem::transmute(b"WM_PROTOCOLS\0"), 0) };
        Self {
            client_window_to_item_idx: Default::default(),
            bindings: Default::default(),
            on_point_changed,
            layout: Layout::new(bounds, ContainerDataConstructor { display, root }, 6),
            point: ItemIdx::Container(0),
            cursor: None,
            focused: None,
            struts: Default::default(),
            struts_frontier: MutableAntichain::new(),
            current_strut: Default::default(),
            root_size: AreaSize {
                height: 0,
                width: 0,
            },

            display,
            root,
            delete_window_atom,
            protocols_atom,
        }
    }

    fn compute_and_set_strut(&mut self) -> bool {
        let mut meet = StrutPartial::default();
        for x in self.struts_frontier.frontier().iter() {
            meet.meet_assign(x)
        }
        if meet == self.current_strut {
            false
        } else {
            self.current_strut = meet;
            true
        }
    }
    /// Returns true iff the strut meet (infimum) changed.
    pub fn record_strut(&mut self, window: Window, new: StrutPartial) -> bool {
        let old = self.struts.insert(window, new);
        let changes = if let Some(old) = old {
            self.struts_frontier.update_iter(vec![(old, -1), (new, 1)])
        } else {
            self.struts_frontier.update_iter(Some((new, 1)))
        };
        drop(changes);
        self.compute_and_set_strut()
    }

    /// Returns true iff the strut meet (infimum) changed.
    pub fn clear_strut(&mut self, window: Window) -> bool {
        if let Some(old) = self.struts.remove(&window) {
            let changes = self.struts_frontier.update_iter(Some((old, -1)));
            drop(changes);
            self.compute_and_set_strut()
        } else {
            false
        }
    }

    pub fn supports_wm_delete(&self, window: x11::xlib::Window) -> bool {
        let mut atoms = null_mut();
        let mut count = 0;
        let status = unsafe { XGetWMProtocols(self.display, window, &mut atoms, &mut count) };
        assert_ne!(status, 0);
        // Let RAII handle freeing the atoms
        let boxed = unsafe {
            let slice = std::slice::from_raw_parts_mut(atoms, count.try_into().unwrap());
            Box::from_raw(slice)
        };
        boxed.iter().any(|&atom| atom == self.delete_window_atom)
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
            info!("Running action: {:?}", action);
            self.update_for_action(action);
        }
        unsafe {
            self.update_point_and_cursor(old_point, new_point, old_cursor, new_cursor);
        }
    }
    pub fn update_for_action(&mut self, action: LayoutAction<WindowData, ContainerData>) {
        match action {
            LayoutAction::NewBounds { idx, bounds } => match idx {
                ItemIdx::Window(w_idx) => {
                    if self
                        .layout
                        .try_window_data(w_idx)
                        .map(|md| md.client.is_some())
                        .unwrap_or(false)
                    {
                        unsafe {
                            self.update_window_bounds(w_idx);
                        }
                    }
                    if let Some(data) = self.layout.try_window_data(w_idx) {
                        unsafe {
                            configure_decorations(
                                self.display,
                                bounds,
                                &data.decorations,
                                &data.template,
                            );
                        }
                    }
                }
                ItemIdx::Container(c_idx) => {
                    let data = self.layout.try_container_data(c_idx).unwrap();
                    unsafe {
                        configure_decorations(
                            self.display,
                            bounds,
                            &data.decorations,
                            &data.template,
                        );
                    }
                }
            },
            LayoutAction::ItemDestroyed { item } => {
                if let ItemAndData::Window(idx, _) = &item {
                    if self.focused == Some(*idx) {
                        self.focused = None;
                    }
                }
                if self
                    .cursor
                    .map(|cursor| !self.layout.is_cursor_valid(cursor))
                    .unwrap_or(false)
                {
                    self.cursor = None;
                }
                match item {
                    ItemAndData::Window(_, data) => unsafe {
                        if let Some(client) = data.client {
                            self.kill_window(client.window);
                        }
                        self.kill_window(data.decorations.down);
                        self.kill_window(data.decorations.up);
                        self.kill_window(data.decorations.right);
                        self.kill_window(data.decorations.left);
                    },
                    ItemAndData::Container(_, data) => unsafe {
                        self.kill_window(data.decorations.down);
                        self.kill_window(data.decorations.up);
                        self.kill_window(data.decorations.right);
                        self.kill_window(data.decorations.left);
                    },
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
    pub fn try_template_mut(&mut self, item: ItemIdx) -> Option<&mut WindowDecorationsTemplate> {
        match item {
            ItemIdx::Window(w) => self
                .layout
                .try_window_data_mut(w)
                .map(|wd| &mut wd.template),
            ItemIdx::Container(c) => self
                .layout
                .try_container_data_mut(c)
                .map(|cd| &mut cd.template),
        }
    }
    pub fn try_template(&self, item: ItemIdx) -> Option<&WindowDecorationsTemplate> {
        match item {
            ItemIdx::Window(w) => self.layout.try_window_data(w).map(|wd| &wd.template),
            ItemIdx::Container(c) => self.layout.try_container_data(c).map(|cd| &cd.template),
        }
    }
    pub fn try_decorations(&self, item: ItemIdx) -> Option<&WindowDecorations> {
        match item {
            ItemIdx::Window(w) => self.layout.try_window_data(w).map(|wd| &wd.decorations),
            ItemIdx::Container(c) => self.layout.try_container_data(c).map(|cd| &cd.decorations),
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
    error!("X error: {:?}", *ev);
    0
}

unsafe extern "C" fn x_io_err(_display: *mut Display) -> i32 {
    let e = std::io::Error::last_os_error();
    error!("X io error (last: {:?})", e);
    0
}

const XLIB_CONN: Token = Token(0);
const FEEDBACK: Token = Token(1);

use mio::unix::pipe::Sender as MioSender;

static FEEDBACK_TX: once_cell::sync::OnceCell<MioSender> = once_cell::sync::OnceCell::new();

#[derive(Debug, Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
struct StrutPartial {
    left: c_ulong,
    right: c_ulong,
    top: c_ulong,
    bottom: c_ulong,
    // left_start_y: c_ulong,
    // left_end_y: c_ulong,
    // right_start_y: c_ulong,
    // right_end_y: c_ulong,
    // top_start_x: c_ulong,
    // top_end_x: c_ulong,
    // bottom_start_x: c_ulong,
    // bottom_end_x: c_ulong,
}

// A strut implies another strut if it blocks _more_ of the screen,
// so these are the dual of what one might expect!
impl timely::PartialOrder for StrutPartial {
    fn less_equal(&self, other: &Self) -> bool {
        self.left >= other.left
            && self.right >= other.right
            && self.top >= other.top
            && self.bottom >= other.bottom
    }
}

// A strut implies another strut if it blocks _more_ of the screen,
// so these are the dual of what one might expect!
impl Lattice for StrutPartial {
    fn join(&self, other: &Self) -> Self {
        Self {
            left: self.left.min(other.left),
            right: self.right.min(other.right),
            top: self.top.min(other.top),
            bottom: self.bottom.min(other.bottom),
        }
    }

    fn meet(&self, other: &Self) -> Self {
        Self {
            left: self.left.max(other.left),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

unsafe fn is_dock(display: *mut Display, window: Window) -> bool {
    let mut n_items = 0;
    let mut bytes_after_return = 0;
    let mut p_result: *mut c_uchar = null_mut();
    let mut actual_type: Atom = 0;
    let mut actual_format: c_int = 0;

    if Success as c_int
        == XGetWindowProperty(
            display,
            window,
            XInternAtom(display, c(b"_NET_WM_WINDOW_TYPE\0"), 0),
            0,
            1,
            0,
            AnyPropertyType as u64,
            &mut actual_type,
            &mut actual_format,
            &mut n_items,
            &mut bytes_after_return,
            &mut p_result,
        )
        && !p_result.is_null()
    {
        let result = std::ptr::read(p_result as *const Atom);
        result == XInternAtom(display, c(b"_NET_WM_WINDOW_TYPE_DOCK\0"), 0)
    } else {
        false
    }
}

unsafe fn get_strut(display: *mut Display, window: Window) -> Option<StrutPartial> {
    let mut n_items = 0;
    let mut bytes_after_return = 0;
    let mut p_strut_partial: *mut c_uchar = null_mut();
    let mut actual_type: Atom = 0;
    let mut actual_format: c_int = 0;
    let expected_items = (size_of::<StrutPartial>() / size_of::<c_ulong>()) as i64;
    info!("calling get_strut on window {}", window);
    if Success as c_int
        == XGetWindowProperty(
            display,
            window,
            XInternAtom(display, c(b"_NET_WM_STRUT_PARTIAL\0"), 0),
            0,
            expected_items,
            0,
            AnyPropertyType as u64,
            &mut actual_type,
            &mut actual_format,
            &mut n_items,
            &mut bytes_after_return,
            &mut p_strut_partial,
        )
        && expected_items == n_items as i64
    {
        {
            let mut r = std::slice::from_raw_parts(p_strut_partial, size_of::<StrutPartial>());
            while let Ok(u) = r.read_u64::<NativeEndian>() {
                info!("Decoded u64: 0x{:x}", u);
            }
        }
        let sp: StrutPartial = std::ptr::read(p_strut_partial as *const StrutPartial);
        XFree(p_strut_partial as *mut c_void);
        Some(sp)
    } else {
        None
    }
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
    let on_client_destroyed = scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"on-client-destroyed\0")),
    );
    let on_point_changed = ProtectedScm(scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"on-point-changed\0")),
    ));
    let on_button1_pressed = scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"on-button1-pressed\0")),
    );
    let after_start = scm_assq_ref(
        config,
        scm_from_utf8_symbol(std::mem::transmute(b"after-start\0")),
    );
    let display = XOpenDisplay(null());
    assert!(!display.is_null());
    XSetErrorHandler(Some(x_err));
    XSetIOErrorHandler(Some(x_io_err));
    let root = XDefaultRootWindow(display);
    XSelectInput(
        display,
        root,
        StructureNotifyMask | SubstructureRedirectMask | SubstructureNotifyMask,
    );
    XSync(display, 0);

    let n_screens = XScreenCount(display);
    assert!(n_screens > 0);
    let screen = XScreenOfDisplay(display, 0);
    let screen = std::ptr::read(screen);

    let root_size = AreaSize {
        width: screen.width.try_into().unwrap(),
        height: screen.height.try_into().unwrap(),
    };
    let root_bounds = WindowBounds {
        position: Default::default(),
        content: root_size,
    };

    let mut wm = WmState::new(display, root, root_bounds, on_point_changed);
    wm.root_size = root_size;

    wm.do_and_recompute(|_wm| {
        Some(LayoutAction::NewBounds {
            idx: ItemIdx::Container(0),
            bounds: root_bounds,
        })
    });

    let wm_scm = make_foreign_object_from_ref(&mut wm, WM_STATE_TYPE);

    insert_bindings(wm_scm.inner, bindings);

    // XGrabServer(display);
    // ... rehome windows ...
    // XUngrabServer(display);

    let on_destroy = |wm: &mut WmState, window| {
        if let Entry::Occupied(oe) = wm.client_window_to_item_idx.entry(window) {
            let idx = oe.remove();
            if wm.layout.exists(ItemIdx::Window(idx)) {
                wm.layout
                    .try_data_mut(ItemIdx::Window(idx))
                    .unwrap()
                    .unwrap_window()
                    .client = None;
            }
            let point = ItemIdx::Window(idx)
                .serialize(Serializer::default())
                .expect("XXX");
            scm_apply_2(on_client_destroyed, wm_scm.inner, point, SCM_EOL);
        }
        if wm.clear_strut(window) {
            wm.do_resize()
        }
    };
    let (feedback_tx, mut feedback_rx) = mio::unix::pipe::new().unwrap();
    FEEDBACK_TX.set(feedback_tx).expect("already ran run_wm!");
    do_on_main_thread(after_start);

    let display_fd = XConnectionNumber(display) as RawFd;
    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1024);
    poll.registry()
        .register(
            &mut SourceFd(&display_fd),
            XLIB_CONN,
            Interest::READABLE | Interest::WRITABLE,
        )
        .unwrap();
    poll.registry()
        .register(&mut feedback_rx, FEEDBACK, Interest::READABLE)
        .unwrap();

    XGrabButton(
        display,
        Button1,
        0,
        root,
        0,
        (ButtonPressMask | ButtonReleaseMask) as u32,
        GrabModeSync,
        GrabModeAsync,
        0,
        0,
    );
    loop {
        let mut e = MaybeUninit::<XEvent>::uninit();
        while poll.poll(&mut events, None).is_err() {}
        for mio_ev in &events {
            if mio_ev.token() == FEEDBACK {
                while let Ok(f) = feedback_rx.read_u64::<NativeEndian>() {
                    let f = f as SCM;
                    scm_apply_1(f, wm_scm.inner, SCM_EOL);
                }
            }
        }
        while XPending(display) > 0 {
            XNextEvent(display, e.as_mut_ptr());
            let e = e.assume_init();
            info!("Event: {:?}", e);
            match e.type_ {
                x11::xlib::KeyPress => {
                    let XKeyEvent { keycode, state, .. } = e.key;
                    let keysym = XKeycodeToKeysym(display, keycode.try_into().unwrap(), 0); // TODO - figure out what the zero means here.

                    let combo = KeyCombo::from_x(keysym, state);
                    info!("received key combo: {:?}", combo);
                    let binding = {
                        let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                        wm.bindings.get(&combo)
                    };
                    if let Some(ProtectedScm(proc)) = binding {
                        info!("binding found, calling into scheme");
                        scm_apply_1(*proc, wm_scm.inner, SCM_EOL);
                    } else {
                        info!("No binding found");
                    };
                }
                x11::xlib::ClientMessage => {
                    let XClientMessageEvent { message_type, .. } = e.client_message;
                    let p_message_type = XGetAtomName(display, message_type);
                    if p_message_type != null_mut() {
                        let message_type = CStr::from_ptr(p_message_type);
                        let s = message_type.to_string_lossy();
                        info!("Client message: {}", s)
                    }
                }
                x11::xlib::ButtonPress => {
                    let XButtonEvent {
                        display,
                        time,
                        x_root,
                        y_root,
                        ..
                    } = e.button;
                    let point = {
                        let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                        let position = Position {
                            x: x_root as usize,
                            y: y_root as usize,
                            root_ctr: wm.layout.displayed_root(),
                        };
                        let w_idx = wm.layout.window_at(position);
                        let point = w_idx.map(|w_idx| ItemIdx::Window(w_idx));
                        info!(
                            "Button pressed at position {:?}, corresponding point {:?}",
                            position, point
                        );
                        point
                    };
                    scm_apply_2(
                        on_button1_pressed,
                        wm_scm.inner,
                        point.serialize(Serializer::default()).unwrap(),
                        SCM_EOL,
                    );
                    // https://stackoverflow.com/questions/46288251/capture-button-events-in-xlib-then-passing-the-event-to-the-client
                    XAllowEvents(display, ReplayPointer, time);
                    XSync(display, 0);
                }
                x11::xlib::ConfigureRequest => {
                    // Let windows do whatever they want if we haven't taken them over yet.
                    let ev = e.configure_request;
                    let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
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
                            let WindowBounds { content, position } = wm.layout.bounds(idx);

                            let inner_size = outer_to_inner_size(content, &data.template);

                            let ev2 = XConfigureEvent {
                                type_: ConfigureNotify,
                                serial: 0,
                                send_event: 1,
                                display,
                                event: ev.window,
                                window: ev.window,
                                x: (position.x + data.template.left.width).try_into().unwrap(),
                                y: (position.y + data.template.up.width).try_into().unwrap(),
                                width: inner_size.width.try_into().unwrap(),
                                height: inner_size.height.try_into().unwrap(),
                                border_width: 0,
                                above: 0,
                                override_redirect: 0, // ??? XXX
                            };
                            let mut ev2 = XEvent { configure: ev2 };

                            // TODO - error handling?
                            XSendEvent(
                                display,
                                ev.window,
                                1,
                                StructureNotifyMask,
                                &mut ev2 as *mut XEvent,
                            );
                        }
                    }
                }

                x11::xlib::ConfigureNotify => {
                    let XConfigureEvent {
                        window,
                        width,
                        height,
                        ..
                    } = e.configure;

                    if window == root {
                        let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                        // let new_bounds = WindowBounds {
                        //     position: Default::default(),
                        //     content: AreaSize {
                        //         width: width.try_into().unwrap(),
                        //         height: height.try_into().unwrap(),
                        //     },
                        // };
                        let new_size = AreaSize {
                            width: width.try_into().unwrap(),
                            height: height.try_into().unwrap(),
                        };
                        // if new_bounds != wm.layout.root_bounds() {
                        //     wm.do_and_recompute(|wm| wm.layout.resize(new_bounds))
                        // }
                        if new_size != wm.root_size {
                            wm.root_size = new_size;
                            wm.do_resize()
                        }
                    }
                }

                x11::xlib::MapRequest => {
                    let XMapRequestEvent { window, .. } = e.map_request;

                    let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                    let already_mapped = wm.client_window_to_item_idx.contains_key(&window);

                    let is_dock = is_dock(display, window);
                    info!("is_dock: {}", is_dock);
                    if !already_mapped && !is_dock {
                        let insert_cursor = scm_apply_1(place_new_window, wm_scm.inner, SCM_EOL);
                        let insert_cursor =
                            MoveOrReplace::deserialize(Deserializer { scm: insert_cursor })
                                .expect("XXX");
                        wm.do_and_recompute(|wm| match insert_cursor {
                            MoveOrReplace::Move(insert_cursor) => {
                                let decorations = make_decorations(display, root);
                                let w_idx = wm.layout.alloc_window(WindowData {
                                    client: Some(X11ClientWindowData {
                                        window,
                                        mapped: false,
                                    }),
                                    decorations,
                                    template: BASIC_DECO,
                                });
                                wm.client_window_to_item_idx.insert(window, w_idx);
                                let actions =
                                    wm.layout.r#move(ItemIdx::Window(w_idx), insert_cursor);
                                wm.point = ItemIdx::Window(w_idx);
                                XRaiseWindow(wm.display, window);
                                actions
                            }
                            MoveOrReplace::Replace(ItemIdx::Window(w_idx)) => {
                                let old_bounds = wm.layout.bounds(ItemIdx::Window(w_idx));

                                wm.client_window_to_item_idx.insert(window, w_idx);
                                wm.point = ItemIdx::Window(w_idx);
                                let old_client = std::mem::replace(
                                    &mut wm
                                        .layout
                                        .try_data_mut(wm.point)
                                        .unwrap()
                                        .unwrap_window()
                                        .client,
                                    Some(X11ClientWindowData {
                                        window,
                                        mapped: false,
                                    }),
                                );
                                XRaiseWindow(wm.display, window);
                                if let Some(X11ClientWindowData { window, mapped: _ }) = old_client
                                {
                                    XDestroyWindow(wm.display, window);
                                }
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
                    let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                    let ev = e.map;
                    if let Some(&idx) = wm.client_window_to_item_idx.get(&ev.window) {
                        if let Some(WindowData { client, .. }) = wm.layout.try_window_data_mut(idx)
                        {
                            let mut client = client
                                .as_mut()
                                .expect("Layout out of sync with client_window_to_item_idx");
                            client.mapped = true;
                        }
                    } else {
                        // Mapping was never requested -- is this a dock/bar ? Check strut property to see.
                        let strut = get_strut(display, ev.window);
                        info!("MapNotify without request. Strut: {:?}", strut);
                        if let Some(strut) = strut {
                            if wm.record_strut(ev.window, strut) {
                                wm.do_resize();
                            }
                        }
                    }
                    wm.ensure_focus();
                }
                x11::xlib::UnmapNotify => {
                    let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                    let ev = e.unmap;
                    if let Some(w_idx) = wm.client_window_to_item_idx.get(&ev.window) {
                        if let Some(client) = wm
                            .layout
                            .try_window_data_mut(*w_idx)
                            .and_then(|data| data.client.as_mut())
                        {
                            client.mapped = false;
                        }
                    }
                    wm.ensure_focus();
                }
                x11::xlib::DestroyNotify => {
                    let XDestroyWindowEvent { window, .. } = e.destroy_window;
                    let wm = get_foreign_object::<WmState>(wm_scm.inner, WM_STATE_TYPE);
                    on_destroy(wm, window);
                }
                _ => {}
            }
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
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    match SpatialDir::deserialize(Deserializer { scm: dir }).expect("XXX") {
        SpatialDir::Planar(dir) => {
            wm.navigate(dir);
        }
        SpatialDir::Parent => {
            if let Some(parent_ctr) = wm.layout.parent_container(wm.point) {
                wm.do_and_recompute(|wm| {
                    wm.point = ItemIdx::Container(parent_ctr);
                    None
                });
            }
        }
        SpatialDir::Child => {
            if let ItemIdx::Container(c_idx) = wm.point {
                let children = wm.layout.children(c_idx);
                if let Some(&(_weight, item)) = children.get(0) {
                    wm.do_and_recompute(|wm| {
                        wm.point = item;
                        None
                    });
                }
            }
        }
    }
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

unsafe extern "C" fn get_point(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.point.serialize(Serializer::default()).expect("XXX")
}

unsafe extern "C" fn set_point(state: SCM, point: SCM) -> SCM {
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.do_and_recompute(|wm| {
        wm.point = point;
        None
    });
    SCM_UNSPECIFIED
}

unsafe extern "C" fn get_cursor(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    wm.cursor.serialize(Serializer::default()).expect("XXX")
}

unsafe extern "C" fn set_cursor(state: SCM, cursor: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let cursor = Option::<MoveCursor>::deserialize(Deserializer { scm: cursor }).expect("XXX");
    if let Some(cursor) = cursor {
        assert!(wm.layout.is_cursor_valid(cursor), "XXX");
    }
    wm.do_and_recompute(|wm| {
        wm.cursor = cursor;
        None
    });
    SCM_UNSPECIFIED
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
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    info!("Killing item at {:?}", point);
    wm.do_and_recompute(|wm| {
        let topo_next = wm.layout.topological_next(wm.point);
        let actions = wm.layout.destroy(point);
        if !wm.layout.exists(wm.point) {
            wm.point = topo_next.unwrap_or_else(|| wm.layout.topological_last());
        }
        actions
    });
    SCM_UNSPECIFIED
}

unsafe extern "C" fn request_kill_client_at(state: SCM, window: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let window = scm_to_uint64(window).try_into().unwrap();
    if let Some(client) = wm.layout.try_window_data(window).expect("XXX").client {
        if wm.supports_wm_delete(client.window) {
            let mut cmd: ClientMessageData = Default::default();
            cmd.set_long(0, wm.delete_window_atom.try_into().unwrap());
            let client_message = XClientMessageEvent {
                type_: ClientMessage,
                serial: 0,
                send_event: 0,
                display: wm.display,
                window: client.window,
                message_type: wm.protocols_atom,
                format: 32,
                data: cmd,
            };
            let mut ev = XEvent { client_message };
            XSendEvent(wm.display, client.window, 0, 0, &mut ev);
        } else {
            todo!()
        }
    }
    SCM_UNSPECIFIED
}

unsafe extern "C" fn new_window_at(state: SCM, cursor: SCM) -> SCM {
    let cur = MoveOrReplace::deserialize(Deserializer { scm: cursor }).expect("XXX");
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let decorations = make_decorations(wm.display, wm.root);
    let win = wm.layout.alloc_window(WindowData {
        client: None,
        decorations,
        template: BASIC_DECO,
    });
    match cur {
        MoveOrReplace::Move(cur) => {
            wm.do_and_recompute(|wm| wm.layout.r#move(ItemIdx::Window(win), cur));
        }
        MoveOrReplace::Replace(_) => todo!(),
    }
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

unsafe extern "C" fn get_layout(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let scm = wm.layout.serialize(Serializer::default()).unwrap();
    scm
}

unsafe extern "C" fn set_focus(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let maybe_window: Option<usize> =
        Deserialize::deserialize(Deserializer { scm: point }).expect("XXX");
    if let Some(window) = maybe_window {
        assert!(wm.layout.exists(ItemIdx::Window(window))); // XXX
    }
    wm.focused = maybe_window;
    wm.ensure_focus();
    SCM_UNSPECIFIED
}

unsafe extern "C" fn nth_child(state: SCM, container: SCM, index: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let container = usize::deserialize(Deserializer { scm: container }).expect("XXX");
    let index = usize::deserialize(Deserializer { scm: index }).expect("XXX");
    let cl = ChildLocation { container, index };
    let item = wm.layout.item_from_child_location(cl);
    let scm = item.serialize(Serializer::default()).unwrap();
    scm
}

unsafe extern "C" fn child_location(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let loc = wm.layout.child_location(point).unwrap_or(ChildLocation {
        container: 0,
        index: 0,
    });
    let scm = loc.serialize(Serializer::default()).unwrap();
    scm
}

unsafe extern "C" fn move_point_to_cursor(state: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    if let Some(cursor) = wm.cursor {
        if !wm.layout.is_ancestor(wm.point, cursor.item()) {
            wm.do_and_recompute(|wm| {
                let actions = wm.layout.r#move(wm.point, cursor);
                actions
            });
        }
    }
    SCM_UNSPECIFIED
}

unsafe extern "C" fn all_descendants(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let iter = wm.layout.iter_descendants(point);
    let result_list = serde::Serializer::collect_seq(Serializer::default(), iter).unwrap();
    result_list
}

unsafe extern "C" fn do_on_main_thread(f: SCM) -> SCM {
    let mut tx = FEEDBACK_TX.get().expect("wm not initialized!");
    tx.write_u64::<NativeEndian>(f as u64).unwrap();
    SCM_UNSPECIFIED
}

unsafe extern "C" fn set_length(state: SCM, point: SCM, length: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    let length = match usize::deserialize(Deserializer { scm: length }) {
        Ok(length) => length,
        Err(_e) => {
            isize::deserialize(Deserializer { scm: length }).unwrap();
            0
        }
    };
    let length = if length == 0 { 1 } else { length };
    wm.do_and_recompute(|wm| wm.layout.set_content_length(point, length));
    SCM_UNSPECIFIED
}

unsafe extern "C" fn get_length(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    match wm.layout.get_content_length(point) {
        Some(length) => length.serialize(Serializer::default()).unwrap(),
        None => SCM_BOOL_F,
    }
}

unsafe extern "C" fn equalize_lengths(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    if let ItemIdx::Container(c_idx) = point {
        wm.do_and_recompute(|wm| wm.layout.equalize_container_children(c_idx))
    }
    SCM_UNSPECIFIED
}

unsafe extern "C" fn toggle_map(state: SCM, point: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let point = ItemIdx::deserialize(Deserializer { scm: point }).expect("XXX");
    if let ItemIdx::Window(w_idx) = point {
        if let Some(client) = wm
            .layout
            .try_window_data(w_idx)
            .and_then(|data| data.client.as_ref())
        {
            if client.mapped {
                wm.request_unmap(client.window)
            } else {
                wm.request_map(client.window)
            }
        }
    }

    SCM_UNSPECIFIED
}

unsafe extern "C" fn _debug_force_resize(state: SCM, width: SCM, height: SCM) -> SCM {
    let wm = get_foreign_object::<WmState>(state, WM_STATE_TYPE);
    let width = usize::deserialize(Deserializer { scm: width }).expect("XXX");
    let height = usize::deserialize(Deserializer { scm: height }).expect("XXX");
    wm.root_size = AreaSize { width, height };
    wm.do_resize();
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
    WM_STATE_TYPE = scm_make_foreign_object_type(wm_name, wm_slots, None);

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
    let c = CStr::from_bytes_with_nul(b"fwm-set-point\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, set_point as *mut c_void);
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
    let c = CStr::from_bytes_with_nul(b"fwm-get-layout\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, get_layout as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-set-focus\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, set_focus as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-clear-bindings\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, clear_bindings as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-set-cursor\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, set_cursor as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-get-cursor\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, get_cursor as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-nth-child\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 3, 0, 0, nth_child as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-child-location\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, child_location as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-move-point-to-cursor\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, move_point_to_cursor as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-new-window-at\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, new_window_at as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-all-descendants\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, all_descendants as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-request-kill-client-at\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, request_kill_client_at as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-mt\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 1, 0, 0, do_on_main_thread as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-set-length\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 3, 0, 0, set_length as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-get-length\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, get_length as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-equalize-lengths\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, equalize_lengths as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-toggle-map\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 2, 0, 0, toggle_map as *mut c_void);
    let c = CStr::from_bytes_with_nul(b"fwm-DEBUG-force-resize\0").unwrap();
    scm_c_define_gsubr(c.as_ptr(), 3, 0, 0, _debug_force_resize as *mut c_void);
    std::ptr::null_mut()
}

fn c(s: &[u8]) -> *const c_char {
    CStr::from_bytes_with_nul(s).unwrap().as_ptr()
}

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    init: String,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let mut init = args.init;
    init.push(0 as char);
    unsafe {
        let old_umask = umask(0);
        if old_umask & 7 == 7 {
            panic!("Umask is {:o} -- anyone is able to access the command socket and control the system! Refusing to run.", old_umask);
        }
        umask(old_umask);
        let mut socket_path = std::env::temp_dir();
        socket_path.push(format!("fwm.{}", std::process::id()));
        info!("Socket path: {}", socket_path.display());
        let listen_arg = format!(
            "--listen={}\0",
            socket_path
                .as_os_str()
                .to_str()
                .expect("Weird characters in TMPDIR ????")
        );

        let args = &[
            c(b"fwm-client\0"),
            c(listen_arg.as_bytes()),
            c(init.as_bytes()),
            null(),
        ];
        scm_with_guile(Some(scheme_setup), null_mut());
        // XXX is this sound? Can argv be modified by guile?
        scm_shell(args.len() as i32 - 1, args.as_ptr() as *mut *mut c_char);
    }
}
