//! ASIO, as a DAW uses it: the audio interface's own driver, its input and outputs on one clock, a buffer the size
//! the player chooses, no Windows mixing between (`announce`'s ASIO engine). The drivers are COM objects Windows
//! lists under `HKLM\SOFTWARE\ASIO`, their interface (IASIO) declared here as the ASIO SDK's header describes it.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows::Win32::System::Registry::{HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW};
use windows::core::{GUID, HRESULT, HSTRING, PCWSTR, PWSTR};

/// The ASIO drivers installed, by name.
pub fn drivers() -> Vec<String> {
    let mut names = Vec::new();
    // SAFETY: a key of ours, read into buffers of ours, closed once read.
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, windows::core::w!("SOFTWARE\\ASIO"), None, KEY_READ, &mut key).is_err() {
            return names;
        }
        for index in 0.. {
            let mut name = [0u16; 256];
            let mut size = name.len() as u32;
            if RegEnumKeyExW(key, index, Some(PWSTR(name.as_mut_ptr())), &mut size, None, None, None, None).is_err() {
                break;
            }
            names.push(String::from_utf16_lossy(&name[..size as usize]));
        }
        let _ = RegCloseKey(key);
    }
    names
}

fn clsid(name: &str) -> Option<GUID> {
    let path = HSTRING::from(format!("SOFTWARE\\ASIO\\{name}"));
    let text = westplay_setup::places::registry_text(HKEY_LOCAL_MACHINE, PCWSTR(path.as_ptr()), windows::core::w!("CLSID"))?;
    GUID::try_from(text.trim().trim_start_matches('{').trim_end_matches('}')).ok()
}

#[repr(C)]
struct IAsio {
    vtbl: *const Vtbl,
}

type This = *mut IAsio;

#[repr(C)]
struct Vtbl {
    query_interface: unsafe extern "system" fn(This, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(This) -> u32,
    release: unsafe extern "system" fn(This) -> u32,
    init: unsafe extern "system" fn(This, *mut c_void) -> i32,
    get_driver_name: unsafe extern "system" fn(This, *mut u8),
    get_driver_version: unsafe extern "system" fn(This) -> i32,
    get_error_message: unsafe extern "system" fn(This, *mut u8),
    start: unsafe extern "system" fn(This) -> i32,
    stop: unsafe extern "system" fn(This) -> i32,
    get_channels: unsafe extern "system" fn(This, *mut i32, *mut i32) -> i32,
    get_latencies: unsafe extern "system" fn(This, *mut i32, *mut i32) -> i32,
    get_buffer_size: unsafe extern "system" fn(This, *mut i32, *mut i32, *mut i32, *mut i32) -> i32,
    can_sample_rate: unsafe extern "system" fn(This, f64) -> i32,
    get_sample_rate: unsafe extern "system" fn(This, *mut f64) -> i32,
    set_sample_rate: unsafe extern "system" fn(This, f64) -> i32,
    get_clock_sources: unsafe extern "system" fn(This, *mut c_void, *mut i32) -> i32,
    set_clock_source: unsafe extern "system" fn(This, i32) -> i32,
    get_sample_position: unsafe extern "system" fn(This, *mut c_void, *mut c_void) -> i32,
    get_channel_info: unsafe extern "system" fn(This, *mut ChannelInfo) -> i32,
    create_buffers: unsafe extern "system" fn(This, *mut BufferInfo, i32, i32, *const Callbacks) -> i32,
    dispose_buffers: unsafe extern "system" fn(This) -> i32,
    control_panel: unsafe extern "system" fn(This) -> i32,
    future: unsafe extern "system" fn(This, i32, *mut c_void) -> i32,
    output_ready: unsafe extern "system" fn(This) -> i32,
}

#[repr(C)]
struct ChannelInfo {
    channel: i32,
    is_input: i32,
    is_active: i32,
    group: i32,
    kind: i32,
    name: [u8; 32],
}

#[repr(C)]
struct BufferInfo {
    is_input: i32,
    channel: i32,
    buffers: [*mut c_void; 2],
}

#[repr(C)]
struct Callbacks {
    buffer_switch: extern "C" fn(i32, i32),
    rate_changed: extern "C" fn(f64),
    message: extern "C" fn(i32, i32, *mut c_void, *mut f64) -> i32,
    buffer_switch_time_info: extern "C" fn(*mut c_void, i32, i32) -> *mut c_void,
}

const OK: i32 = 0;

#[link(name = "ole32")]
unsafe extern "system" {
    #[link_name = "CoCreateInstance"]
    fn co_create_instance(clsid: *const GUID, outer: *mut c_void, context: u32, iid: *const GUID, out: *mut *mut c_void)
    -> HRESULT;
    #[link_name = "CoInitializeEx"]
    fn co_initialize_ex(reserved: *mut c_void, flags: u32) -> HRESULT;
}

/// What's running: the interface's rate, the buffer, and its latencies (frames).
#[derive(Clone, Copy, Debug)]
pub struct Running {
    pub rate: u32,
    pub buffer: u32,
    pub input_latency: u32,
    pub output_latency: u32,
}

/// A buffer's worth: the input channel's samples in, the two outputs' out.
pub type Process = Box<dyn FnMut(&[f32], &mut [[f32; 2]]) + Send>;

struct Live {
    process: Process,
    input: ([*mut c_void; 2], i32),
    outputs: [([*mut c_void; 2], i32); 2],
    heard: Vec<f32>,
    played: Vec<[f32; 2]>,
    driver: This,
    output_ready: bool,
}

// SAFETY: the driver's buffers and the driver itself, used only between createBuffers and disposeBuffers, from
// the driver's own callback (and the thread that made them, before and after).
unsafe impl Send for Live {}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);
static RESET: AtomicBool = AtomicBool::new(false);

extern "C" fn buffer_switch(index: i32, _direct: i32) {
    let mut live = LIVE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(live) = live.as_mut() else { return };
    let half = usize::from(index != 0);
    let frames = live.heard.len();
    // SAFETY: each buffer `frames` samples of its kind, the driver's for this half.
    unsafe {
        for (i, sample) in live.heard.iter_mut().enumerate() {
            *sample = read(live.input.1, live.input.0[half], i);
        }
        (live.process)(&live.heard, &mut live.played);
        for (side, (buffers, kind)) in live.outputs.iter().enumerate() {
            for (i, frame) in live.played.iter().enumerate().take(frames) {
                write(*kind, buffers[half], i, frame[side]);
            }
        }
        if live.output_ready {
            ((*(*live.driver).vtbl).output_ready)(live.driver);
        }
    }
}

extern "C" fn rate_changed(_rate: f64) {
    RESET.store(true, Ordering::Release);
}

extern "C" fn message(selector: i32, value: i32, _message: *mut c_void, _opt: *mut f64) -> i32 {
    match selector {
        // Supported: the engine's version, a reset, a resync, latencies changed.
        1 => i32::from(matches!(value, 2 | 3 | 5 | 6)),
        2 => 2,
        3 | 5 => {
            RESET.store(true, Ordering::Release);
            1
        }
        6 => 1,
        _ => 0,
    }
}

extern "C" fn buffer_switch_time_info(params: *mut c_void, index: i32, direct: i32) -> *mut c_void {
    buffer_switch(index, direct);
    params
}

static CALLBACKS: Callbacks = Callbacks { buffer_switch, rate_changed, message, buffer_switch_time_info };

unsafe fn read(kind: i32, buffer: *mut c_void, i: usize) -> f32 {
    // SAFETY: `i` within the buffer, as the caller says.
    unsafe {
        match kind {
            16 => f32::from(*(buffer as *const i16).add(i)) / 32768.0,
            17 => {
                let b = (buffer as *const u8).add(i * 3);
                let v = i32::from(*b) << 8 | i32::from(*b.add(1)) << 16 | i32::from(*b.add(2)) << 24;
                v as f32 / 2_147_483_648.0
            }
            18 => *(buffer as *const i32).add(i) as f32 / 2_147_483_648.0,
            19 => *(buffer as *const f32).add(i),
            20 => *(buffer as *const f64).add(i) as f32,
            24..=27 => {
                let bits = [16, 18, 20, 24][(kind - 24) as usize];
                *(buffer as *const i32).add(i) as f32 / (1i64 << (bits - 1)) as f32
            }
            _ => 0.0,
        }
    }
}

unsafe fn write(kind: i32, buffer: *mut c_void, i: usize, value: f32) {
    let value = value.clamp(-1.0, 1.0);
    // SAFETY: `i` within the buffer, as the caller says.
    unsafe {
        match kind {
            16 => *(buffer as *mut i16).add(i) = (value * 32767.0) as i16,
            17 => {
                let v = (value * 8_388_607.0) as i32;
                let b = (buffer as *mut u8).add(i * 3);
                *b = v as u8;
                *b.add(1) = (v >> 8) as u8;
                *b.add(2) = (v >> 16) as u8;
            }
            18 => *(buffer as *mut i32).add(i) = (f64::from(value) * 2_147_483_647.0) as i32,
            19 => *(buffer as *mut f32).add(i) = value,
            20 => *(buffer as *mut f64).add(i) = f64::from(value),
            24..=27 => {
                let bits = [16, 18, 20, 24][(kind - 24) as usize];
                *(buffer as *mut i32).add(i) = (value * ((1i64 << (bits - 1)) - 1) as f32) as i32;
            }
            _ => {}
        }
    }
}

fn supported(kind: i32) -> bool {
    matches!(kind, 16..=20 | 24..=27)
}

/// The buffer the driver takes nearest `wanted` frames.
fn buffer_size(wanted: i32, min: i32, max: i32, preferred: i32, granularity: i32) -> i32 {
    let wanted = wanted.clamp(min.max(1), max.max(min).max(1));
    match granularity {
        0 => preferred,
        -1 => {
            let mut size = min.max(1);
            while size < wanted && size * 2 <= max {
                size *= 2;
            }
            size
        }
        g if g > 0 => (min + ((wanted - min + g / 2) / g) * g).clamp(min, max),
        _ => preferred,
    }
}

/// Runs the driver `name`: its input `input` (0 = the first) and outputs `output` and the next, at a buffer
/// nearest `buffer` frames, `process` making each buffer's sound, until `going` says no (or the driver asks for
/// a reset). `started` is told what runs. On its own thread (the driver's apartment).
pub fn run(
    name: &str,
    input: u32,
    output: u32,
    buffer: u32,
    process: Process,
    started: &(dyn Fn(Running, Vec<String>, Vec<String>) + Sync),
    going: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let clsid = clsid(name).ok_or_else(|| format!("the ASIO driver {name} isn't installed"))?;
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                // SAFETY: the driver's interface used as the ASIO SDK describes, its buffers between createBuffers
                // and disposeBuffers; released once done.
                unsafe {
                    let _ = co_initialize_ex(std::ptr::null_mut(), 0x2);
                    let mut driver: *mut c_void = std::ptr::null_mut();
                    co_create_instance(&clsid, std::ptr::null_mut(), 1, &clsid, &mut driver)
                        .ok()
                        .map_err(|e| format!("the ASIO driver {name} can't be loaded: {e}"))?;
                    let driver = driver as This;
                    let vt = &*(*driver).vtbl;
                    let result = (|| {
                        if (vt.init)(driver, std::ptr::null_mut()) == 0 {
                            let mut text = [0u8; 128];
                            (vt.get_error_message)(driver, text.as_mut_ptr());
                            let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
                            return Err(format!("the ASIO driver {name} won't start: {}", String::from_utf8_lossy(&text[..end])));
                        }
                        let (mut inputs, mut outputs) = (0, 0);
                        (vt.get_channels)(driver, &mut inputs, &mut outputs);
                        let names = |is_input: bool, count: i32| {
                            (0..count)
                                .map(|channel| {
                                    let mut info = ChannelInfo {
                                        channel,
                                        is_input: i32::from(is_input),
                                        is_active: 0,
                                        group: 0,
                                        kind: 0,
                                        name: [0; 32],
                                    };
                                    (vt.get_channel_info)(driver, &mut info);
                                    let end = info.name.iter().position(|&c| c == 0).unwrap_or(32);
                                    (String::from_utf8_lossy(&info.name[..end]).into_owned(), info.kind)
                                })
                                .collect::<Vec<_>>()
                        };
                        let (ins, outs) = (names(true, inputs), names(false, outputs));
                        let input = input as usize;
                        let output = output as usize;
                        if input >= ins.len() || output >= outs.len() {
                            return Err(format!("the ASIO driver {name} has no input {} or outputs {}", input + 1, output + 1));
                        }
                        let right = (output + 1).min(outs.len() - 1);
                        for kind in [ins[input].1, outs[output].1, outs[right].1] {
                            if !supported(kind) {
                                return Err(format!("the ASIO driver {name}'s sample type {kind} isn't one WestPlay knows"));
                            }
                        }
                        let (mut min, mut max, mut preferred, mut granularity) = (0, 0, 0, 0);
                        (vt.get_buffer_size)(driver, &mut min, &mut max, &mut preferred, &mut granularity);
                        let size = buffer_size(buffer as i32, min, max, preferred, granularity);
                        let mut rate = 0.0;
                        (vt.get_sample_rate)(driver, &mut rate);
                        let mut infos = [
                            BufferInfo { is_input: 1, channel: input as i32, buffers: [std::ptr::null_mut(); 2] },
                            BufferInfo { is_input: 0, channel: output as i32, buffers: [std::ptr::null_mut(); 2] },
                            BufferInfo { is_input: 0, channel: right as i32, buffers: [std::ptr::null_mut(); 2] },
                        ];
                        let count = if right == output { 2 } else { 3 };
                        if (vt.create_buffers)(driver, infos.as_mut_ptr(), count, size, &CALLBACKS) != OK {
                            return Err(format!("the ASIO driver {name} won't make its buffers ({size} frames)"));
                        }
                        let (mut input_latency, mut output_latency) = (0, 0);
                        (vt.get_latencies)(driver, &mut input_latency, &mut output_latency);
                        let output_ready = (vt.output_ready)(driver) == OK;
                        let right_buffers = if count == 3 { infos[2].buffers } else { infos[1].buffers };
                        *LIVE.lock().unwrap_or_else(|e| e.into_inner()) = Some(Live {
                            process,
                            input: (infos[0].buffers, ins[input].1),
                            outputs: [(infos[1].buffers, outs[output].1), (right_buffers, outs[right].1)],
                            heard: vec![0.0; size as usize],
                            played: vec![[0.0; 2]; size as usize],
                            driver,
                            output_ready,
                        });
                        RESET.store(false, Ordering::Release);
                        let result = if (vt.start)(driver) == OK {
                            started(
                                Running {
                                    rate: rate as u32,
                                    buffer: size as u32,
                                    input_latency: input_latency.max(0) as u32,
                                    output_latency: output_latency.max(0) as u32,
                                },
                                ins.into_iter().map(|(name, _)| name).collect(),
                                outs.into_iter().map(|(name, _)| name).collect(),
                            );
                            while going() && !RESET.load(Ordering::Acquire) {
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            (vt.stop)(driver);
                            Ok(())
                        } else {
                            Err(format!("the ASIO driver {name} won't run"))
                        };
                        *LIVE.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        (vt.dispose_buffers)(driver);
                        result
                    })();
                    (vt.release)(driver);
                    result
                }
            })
            .join()
            .unwrap_or_else(|_| Err("the ASIO driver crashed".into()))
    })
}
