// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The server token of golem-game: the frames that are open on a thread, and the token that a
 * call of the contract takes from the innermost one. A thread that holds no frame gets no token,
 * and a token is refused once its frame is closed, so no call is made without an open frame.
 */

use std::{
    cell::RefCell,
    error,
    fmt,
    marker::PhantomData,
    rc::Rc, //
};

use crate::{
    Contract,
    JniFault, //
};

/// One open frame: its serial on its thread, and what answers the calls of the contract in it.
struct Open {
    serial: u64,
    contract: Rc<dyn Contract>,
}

/// The frames of one thread.
struct Frames {
    /// The frames that are open, the innermost last. The depth of a frame is its place in this
    /// list, counted from one.
    open: Vec<Open>,
    /// How many frames the thread opened so far, which is the serial of the youngest.
    opened: u64,
}

thread_local! {
    /// The frames of this thread.
    static FRAMES: RefCell<Frames> = const {
        RefCell::new(Frames {
            open: Vec::new(),
            opened: 0,
        })
    };
}

/// A frame on the thread that opened it, open until it is dropped.
///
/// The frame names what answers the calls of the contract while it is open, and a frame that
/// is opened inside another is the innermost until it closes. Frames close in order: the drop
/// of a frame also closes every frame that was opened inside it and is still open, and a frame
/// that was closed that way does nothing when it is dropped later. A frame that is never
/// dropped stays open until a frame around it closes.
///
/// In the design of okf/api.md section 7 a frame stands for one native call of the bridge mod.
#[must_use = "a frame closes when it is dropped, so a frame that is not kept is never open"]
pub struct Frame {
    depth: usize,
    serial: u64,
    thread: PhantomData<Rc<()>>,
}

impl Frame {
    /// Opens a frame on this thread, whose calls of the contract `contract` answers.
    ///
    /// On a thread that ends and whose thread locals are gone already, the frame is never open.
    pub fn open(contract: Rc<dyn Contract>) -> Self {
        let opened = FRAMES.try_with(|frames| {
            let mut frames = frames.borrow_mut();
            frames.opened += 1;
            let serial = frames.opened;
            frames.open.push(Open { serial, contract });
            (frames.open.len(), serial)
        });
        let (depth, serial) = opened.unwrap_or((0, 0));
        Self {
            depth,
            serial,
            thread: PhantomData,
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        // The frames that this drop closes leave the list first and are dropped once the borrow
        // has ended, since the drop of what answered their calls may ask for the token. On a
        // thread that ends the thread locals may be gone, and then no frame is open any more.
        let closed = FRAMES.try_with(|frames| {
            let mut frames = frames.borrow_mut();
            let index = self.depth.checked_sub(1)?;
            let own = frames.open.get(index)?.serial == self.serial;
            own.then(|| frames.open.split_off(index))
        });
        drop(closed);
    }
}

/// The server token: the right to call the contract, which a thread holds while a frame is
/// open on it.
///
/// A token belongs to the frame it was taken in. It carries the depth and the serial of that
/// frame, and a call with a token whose frame is closed is refused and never made. The
/// lifetime `'t` is nominal: it stands where the borrow of one native call will stand and
/// binds nothing yet, so the check at the call is what holds a token to its frame. The token
/// is neither `Send` nor `Sync`, so it never leaves its thread.
pub struct Server<'t> {
    depth: usize,
    serial: u64,
    native: PhantomData<(&'t (), Rc<()>)>,
}

impl Server<'_> {
    /// Answers what answers the calls of the contract in the frame of this token, or the fault
    /// of a refused call for a token whose frame is closed.
    pub(crate) fn contract(&self) -> Result<Rc<dyn Contract>, JniFault> {
        FRAMES
            .try_with(|frames| {
                let frames = frames.borrow();
                let open = frames.open.get(self.depth.checked_sub(1)?)?;
                (open.serial == self.serial).then(|| Rc::clone(&open.contract))
            })
            .ok()
            .flatten()
            .ok_or_else(JniFault::refused)
    }
}

/// The refusal of a thread that holds no server token, since no frame is open on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotOnServerThread;

impl fmt::Display for NotOnServerThread {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "The call needs the server thread, and the calling thread holds no server token.",
        )
    }
}

impl error::Error for NotOnServerThread {}

/// Answers the server token of this thread, taken in the innermost frame that is open on it,
/// or the refusal of a thread that holds no open frame. A thread that ends and whose thread
/// locals are gone already holds none.
pub fn server<'t>() -> Result<Server<'t>, NotOnServerThread> {
    FRAMES
        .try_with(|frames| {
            let frames = frames.borrow();
            frames.open.last().map(|open| Server {
                depth: frames.open.len(),
                serial: open.serial,
                native: PhantomData,
            })
        })
        .ok()
        .flatten()
        .ok_or(NotOnServerThread)
}
