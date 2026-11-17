//! The player's wheel and button boxes (a Moza R5's rim, any game controller Windows has): their buttons, bound in
//! the companion's Controls to what WestPlay's keybinds do (the music's volume, play/pause, the next and previous
//! song, the screen's brightness), work whenever WestPlay runs, the game in front or not, with or without the iPad.
//!
//! Read with Windows' joystick functions (winmm), which answer for every process whatever window is in front: a
//! thread looks for controllers every few seconds and reads those there 100 times a second, each newly pressed
//! button sent to the host. A controller is named by its USB vendor and product (`346e:0004`), so a binding stays
//! with the wheel whatever order Windows numbers the controllers in.

use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use windows::Win32::Media::Multimedia::{
    JOY_RETURNBUTTONS, JOYCAPSW, JOYERR_NOERROR, JOYINFOEX, joyGetDevCapsW, joyGetPosEx,
};
use windows::Win32::System::Registry::HKEY_CURRENT_USER;
use windows::core::{HSTRING, PCWSTR};

/// Controllers Windows numbers (winmm's 16).
const IDS: u32 = 16;
const READ_EVERY: Duration = Duration::from_millis(10);
const LOOK_EVERY: Duration = Duration::from_secs(3);

/// A button pressed: its controller (vendor:product), the controller's name, and the button (1 = the first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Press {
    pub device: String,
    pub name: String,
    pub button: u32,
    /// Pressed (or, `false`, let go).
    pub down: bool,
}

pub struct Controllers {
    presses: Receiver<Press>,
}

impl Controllers {
    pub fn start() -> Self {
        let (sender, presses) = mpsc::channel();
        let _ = std::thread::Builder::new().name("westplay-controllers".into()).spawn(move || {
            let mut found: Vec<(u32, String, String)> = Vec::new();
            let mut held = [0u32; IDS as usize];
            let mut looked: Option<Instant> = None;
            loop {
                if looked.is_none_or(|at| at.elapsed() >= LOOK_EVERY) {
                    looked = Some(Instant::now());
                    let now: Vec<(u32, String, String)> = (0..IDS).filter_map(|id| connected(id).map(|(device, name)| (id, device, name))).collect();
                    if now.iter().map(|(_, d, _)| d).ne(found.iter().map(|(_, d, _)| d)) {
                        for (_, device, name) in &now {
                            log!("controller: {name} ({device})");
                        }
                    }
                    found = now;
                }
                for (id, device, name) in &found {
                    let Some(buttons) = buttons(*id) else { continue };
                    let changed = buttons ^ held[*id as usize];
                    held[*id as usize] = buttons;
                    for bit in 0..32 {
                        if changed & (1 << bit) != 0 {
                            let down = buttons & (1 << bit) != 0;
                            let press = Press { device: device.clone(), name: name.clone(), button: bit + 1, down };
                            if sender.send(press).is_err() {
                                return;
                            }
                        }
                    }
                }
                std::thread::sleep(READ_EVERY);
            }
        });
        Self { presses }
    }

    /// The buttons pressed since last taken.
    pub fn take(&self) -> Vec<Press> {
        self.presses.try_iter().collect()
    }
}

fn buttons(id: u32) -> Option<u32> {
    let mut info = JOYINFOEX {
        dwSize: std::mem::size_of::<JOYINFOEX>() as u32,
        dwFlags: JOY_RETURNBUTTONS as u32,
        ..Default::default()
    };
    // SAFETY: `info` is ours, its size set as the call asks.
    (unsafe { joyGetPosEx(id, &mut info) } == JOYERR_NOERROR).then_some(info.dwButtons)
}

/// The controller at `id`, if one is there: its vendor:product and name.
fn connected(id: u32) -> Option<(String, String)> {
    buttons(id)?;
    let mut caps = JOYCAPSW::default();
    // SAFETY: `caps` is ours, its size given.
    if unsafe { joyGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<JOYCAPSW>() as u32) } != JOYERR_NOERROR {
        return None;
    }
    // The struct is packed: its fields copied out before they're formatted.
    let (vendor, product, pname) = (caps.wMid, caps.wPid, caps.szPname);
    let device = format!("{vendor:04x}:{product:04x}");
    // Windows' own name for it (the one Game Controllers shows), else the driver's.
    let key = format!(
        "System\\CurrentControlSet\\Control\\MediaProperties\\PrivateProperties\\Joystick\\OEM\\VID_{:04X}&PID_{:04X}",
        vendor, product
    );
    let key = HSTRING::from(key);
    let name = westplay_setup::places::registry_text(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), windows::core::w!("OEMName"))
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            let end = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
            String::from_utf16_lossy(&pname[..end])
        });
    Some((device, name))
}
