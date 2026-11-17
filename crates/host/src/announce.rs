//! The coach's PA: the player's microphone live through the bus's own speakers, heard at once (always, or while a
//! push-to-talk key or wheel button is held), and announcements recorded into slots and played back the same way
//! (the companion's Controls › Announcements). Its audio set up as a DAW's (the companion's Audio settings): the
//! driver (WASAPI shared with every app, WASAPI with the microphone WestPlay's alone, or an interface's ASIO
//! driver), the devices, and the buffer, so the player trades latency against crackle for their own hardware.
//!
//! The voice: bandpassed and driven as the speakers are worn (a PA's microphone and ceiling units), played from every
//! passenger unit along the coach, each later and quieter as far as it is from the driver's head. Recordings are
//! the microphone's own sound (the PA's colour added as they play), kept as WAV files.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, S_OK, WAIT_EVENT, WAIT_OBJECT_0};
use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED, AUDCLNT_SHAREMODE_EXCLUSIVE, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_EVENTCALLBACK, DEVICE_STATE_ACTIVE, EDataFlow, IAudioCaptureClient, IAudioClient, IAudioClient3,
    IAudioRenderClient, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX, eCapture, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, CreateEventW, WaitForMultipleObjects,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::core::{HSTRING, Interface, PCWSTR};

use crate::settings::{AnnounceSettings, AudioDriver};

/// How many announcements can be kept.
pub const SLOTS: usize = 6;
/// The longest recording (seconds).
const LONGEST: f32 = 60.0;

/// One unit the voice comes from: how much later (ms), how loud (1 = the nearest), which side (0 left, 1 right).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    pub delay: f32,
    pub gain: f32,
    pub side: usize,
}

/// A coach's speakers as the voice's taps (the passengers' units, placed from the head).
pub fn taps(speakers: &[crate::speakers::Speaker], head: [f32; 3]) -> Vec<Tap> {
    let distance = |at: [f32; 3]| (at[0] - head[0]).hypot(at[1] - head[1]).hypot(at[2] - head[2]).max(0.3);
    let nearest = speakers.iter().map(|s| distance(s.at)).fold(f32::MAX, f32::min);
    speakers
        .iter()
        .map(|s| Tap {
            delay: s.delay,
            gain: nearest / distance(s.at),
            side: usize::from(s.channel == crate::speakers::Channel::Right),
        })
        .collect()
}

/// An audio device Windows has: its id (for the settings) and name.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Device {
    pub id: String,
    pub name: String,
}

/// What's running, as a DAW's audio settings show it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    /// "WASAPI", "WASAPI exclusive", "ASIO: <driver>".
    pub driver: String,
    pub rate: u32,
    /// The buffer (frames).
    pub buffer: u32,
    pub input_ms: f32,
    pub output_ms: f32,
}

/// What the companion shows: each slot's length (seconds, `None`: empty), what's recording or playing, whether the
/// microphone is heard now, and the audio: the devices and drivers to choose from, what runs, what went wrong.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceStatus {
    pub slots: Vec<Option<f32>>,
    pub recording: Option<usize>,
    pub playing: Option<usize>,
    pub talking: bool,
    pub inputs: Vec<Device>,
    pub outputs: Vec<Device>,
    pub asio_drivers: Vec<String>,
    /// The ASIO driver's channels (its names), once it has run.
    pub asio_inputs: Vec<String>,
    pub asio_outputs: Vec<String>,
    pub engine: Option<Engine>,
    pub error: Option<String>,
}

#[derive(Clone)]
struct Recording {
    rate: u32,
    samples: Arc<Vec<f32>>,
}

#[derive(Default)]
struct State {
    slots: [Option<Recording>; SLOTS],
    /// The slot being recorded, what's in it so far, and its rate (0 until the engine's is known).
    recording: Option<(usize, Vec<f32>, u32)>,
    /// The slot playing, and how far (in its samples).
    playing: Option<(usize, Recording, f64)>,
}

struct Shared {
    settings: Mutex<AnnounceSettings>,
    /// Changes whenever the settings do, for the engine to start again when its own have.
    generation: AtomicU64,
    /// Push-to-talk bound to a wheel button, and that button held.
    wheel_bound: AtomicBool,
    wheel_held: AtomicBool,
    /// The keyboard's push-to-talk key (a virtual key; 0: none).
    key: AtomicU32,
    talking: AtomicBool,
    taps: Mutex<Vec<Tap>>,
    state: Mutex<State>,
    status: Mutex<AnnounceStatus>,
    dir: PathBuf,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn settings(&self) -> AnnounceSettings {
        self.settings.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn status(&self) -> MutexGuard<'_, AnnounceStatus> {
        self.status.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The microphone wanted open: heard always, push-to-talk ready, or recording.
    fn armed(&self) -> bool {
        let settings = self.settings();
        settings.on || settings.ptt_key.is_some() || self.wheel_bound.load(Ordering::Acquire) || self.state().recording.is_some()
    }

    /// The microphone heard now: always, or its push-to-talk held.
    fn talk(&self, on: bool) -> bool {
        let key = self.key.load(Ordering::Acquire);
        // SAFETY: reads the key's state, any thread.
        let key_held = key != 0 && unsafe { GetAsyncKeyState(key as i32) } as u16 & 0x8000 != 0;
        let talking = on || key_held || self.wheel_held.load(Ordering::Acquire);
        self.talking.store(talking, Ordering::Release);
        talking
    }
}

pub struct Announcer {
    shared: Arc<Shared>,
}

impl Announcer {
    /// Its recordings kept in `dir`.
    pub fn start(dir: PathBuf) -> Self {
        let mut state = State::default();
        for (slot, kept) in state.slots.iter_mut().enumerate() {
            *kept = load(&slot_file(&dir, slot));
        }
        let shared = Arc::new(Shared {
            settings: Mutex::new(AnnounceSettings::default()),
            generation: AtomicU64::new(0),
            wheel_bound: AtomicBool::new(false),
            wheel_held: AtomicBool::new(false),
            key: AtomicU32::new(0),
            talking: AtomicBool::new(false),
            taps: Mutex::new(Vec::new()),
            state: Mutex::new(state),
            status: Mutex::new(AnnounceStatus::default()),
            dir,
        });
        let thread = shared.clone();
        let _ = std::thread::Builder::new().name("westplay-announce".into()).spawn(move || {
            // SAFETY: this thread's COM, once.
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let mut listed: Option<Instant> = None;
            loop {
                if listed.is_none_or(|at| at.elapsed() >= Duration::from_secs(5)) {
                    listed = Some(Instant::now());
                    list_devices(&thread);
                }
                let armed = thread.armed();
                if armed || thread.state().playing.is_some() {
                    let result = match thread.settings().driver {
                        AudioDriver::Asio => run_asio(&thread, armed),
                        driver => run_wasapi(&thread, armed, driver == AudioDriver::Exclusive).map_err(|e| e.to_string()),
                    };
                    thread.status().engine = None;
                    if let Err(e) = result {
                        log!("announcements: {e}");
                        thread.status().error = Some(e);
                        thread.state().playing = None;
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        Self { shared }
    }

    /// The companion's settings, and whether push-to-talk has a wheel button.
    pub fn configure(&self, settings: &AnnounceSettings, wheel_bound: bool) {
        let mut current = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner());
        if current.on != settings.on {
            log!("announcements: the microphone {}", if settings.on { "on" } else { "off" });
        }
        if *current != *settings {
            *current = settings.clone();
            self.shared.generation.fetch_add(1, Ordering::AcqRel);
            self.shared.status().error = None;
        }
        self.shared.key.store(settings.ptt_key.map_or(0, |key| key.virtual_key()), Ordering::Release);
        self.shared.wheel_bound.store(wheel_bound, Ordering::Release);
    }

    /// The wheel's push-to-talk button pressed or let go.
    pub fn wheel_talk(&self, held: bool) {
        self.shared.wheel_held.store(held, Ordering::Release);
    }

    pub fn set_taps(&self, taps: Vec<Tap>) {
        *self.shared.taps.lock().unwrap_or_else(|e| e.into_inner()) = taps;
    }

    /// Records into `slot`, or, recording there already, stops and keeps it.
    pub fn record(&self, slot: usize) {
        if slot >= SLOTS {
            return;
        }
        let mut state = self.shared.state();
        let was = state.recording.as_ref().map(|(at, _, _)| *at);
        keep_recording(&self.shared.dir, &mut state);
        if was != Some(slot) {
            state.recording = Some((slot, Vec::new(), 0));
        }
    }

    /// Plays `slot`, or, playing it already, stops it.
    pub fn play(&self, slot: usize) {
        let mut state = self.shared.state();
        if state.playing.as_ref().is_some_and(|(at, _, _)| *at == slot) {
            state.playing = None;
        } else if let Some(recording) = state.slots.get(slot).cloned().flatten() {
            state.playing = Some((slot, recording, 0.0));
        }
    }

    pub fn clear(&self, slot: usize) {
        if slot >= SLOTS {
            return;
        }
        let mut state = self.shared.state();
        state.slots[slot] = None;
        if state.playing.as_ref().is_some_and(|(at, _, _)| *at == slot) {
            state.playing = None;
        }
        let _ = std::fs::remove_file(slot_file(&self.shared.dir, slot));
    }

    pub fn stop(&self) {
        let mut state = self.shared.state();
        state.playing = None;
        keep_recording(&self.shared.dir, &mut state);
    }

    pub fn status(&self) -> AnnounceStatus {
        let mut status = self.shared.status().clone();
        let state = self.shared.state();
        status.slots = state
            .slots
            .iter()
            .map(|slot| slot.as_ref().map(|r| (r.samples.len() as f32 / r.rate.max(1) as f32 * 10.0).round() / 10.0))
            .collect();
        status.recording = state.recording.as_ref().map(|(slot, _, _)| *slot);
        status.playing = state.playing.as_ref().map(|(slot, _, _)| *slot);
        status.talking = self.shared.talking.load(Ordering::Acquire);
        status
    }
}

/// Windows' microphones and speakers, and the ASIO drivers, for the companion's lists.
fn list_devices(shared: &Shared) {
    let list = |flow: EDataFlow| -> Vec<Device> {
        // SAFETY: COM calls on interfaces we hold; each id freed once read.
        unsafe {
            let Ok(devices) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
                return Vec::new();
            };
            let Ok(collection) = devices.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) else { return Vec::new() };
            let count = collection.GetCount().unwrap_or(0);
            (0..count)
                .filter_map(|i| {
                    let device = collection.Item(i).ok()?;
                    let id = device.GetId().ok()?;
                    let text = id.to_string().ok();
                    CoTaskMemFree(Some(id.0 as *const _));
                    let store = device.OpenPropertyStore(STGM_READ).ok()?;
                    let name = store
                        .GetValue(&windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName)
                        .ok()
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    Some(Device { id: text?, name })
                })
                .collect()
        }
    };
    let (inputs, outputs, asio) = (list(eCapture), list(eRender), crate::asio::drivers());
    let mut status = shared.status();
    status.inputs = inputs;
    status.outputs = outputs;
    status.asio_drivers = asio;
}

/// The voice made from what the microphone hears (and the slot playing), a buffer at a time, at the engine's rate.
struct Processor {
    shared: Arc<Shared>,
    voice: Voice,
    rate: u32,
    /// The microphone's level heard (0 to 1), faded in and out over a few milliseconds (no click).
    gate: f32,
    /// The recorded announcements alone, through a PA of their own, for the copy on Windows (ASIO's: Discord hears
    /// them, the live voice only the player, once): each block's, while `copying`.
    copying: bool,
    copy_voice: Voice,
    copied: Vec<[f32; 2]>,
}

impl Processor {
    fn new(shared: Arc<Shared>, rate: u32) -> Self {
        Self {
            shared,
            voice: Voice::new(rate as f32),
            rate,
            gate: 0.0,
            copying: false,
            copy_voice: Voice::new(rate as f32),
            copied: Vec::new(),
        }
    }

    fn block(&mut self, input: &[f32], out: &mut [[f32; 2]]) {
        let settings = self.shared.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
        self.voice.gain = settings.volume.min(100) as f32 / 100.0;
        self.voice.set_quality(settings.quality);
        self.voice.set_taps(&self.shared.taps.lock().unwrap_or_else(|e| e.into_inner()));
        if self.copying {
            self.copy_voice.gain = self.voice.gain;
            self.copy_voice.set_quality(settings.quality);
            self.copy_voice.set_taps(&self.shared.taps.lock().unwrap_or_else(|e| e.into_inner()));
        }
        self.copied.clear();
        let talking = self.shared.talk(settings.on);
        let mut state = self.shared.state();
        let full = match &mut state.recording {
            Some((_, samples, rate)) => {
                *rate = self.rate;
                samples.extend_from_slice(input);
                samples.len() as f32 >= LONGEST * self.rate as f32
            }
            None => false,
        };
        if full {
            keep_recording(&self.shared.dir, &mut state);
        }
        let fade = 1.0 / (0.004 * self.rate as f32);
        for (i, frame) in out.iter_mut().enumerate() {
            self.gate = if talking { (self.gate + fade).min(1.0) } else { (self.gate - fade).max(0.0) };
            let x = input.get(i).copied().unwrap_or(0.0) * self.gate;
            let mut recorded = 0.0;
            let mut playing = false;
            if let Some((_, recording, at)) = &mut state.playing {
                recorded = recording.samples.get(*at as usize).copied().unwrap_or(0.0);
                *at += f64::from(recording.rate) / f64::from(self.rate);
                playing = true;
            }
            *frame = self.voice.play(x + recorded, playing || self.gate > 0.0);
            if self.copying {
                self.copied.push(self.copy_voice.play(recorded, playing));
            }
        }
        if state.playing.as_ref().is_some_and(|(_, recording, at)| *at as usize >= recording.samples.len()) {
            state.playing = None;
        }
    }
}

/// The engine runs on while it's needed as it is: the microphone wanted as when it started (or playing without),
/// and the settings it started with unchanged.
fn going(shared: &Shared, armed: bool, generation: u64) -> bool {
    shared.generation.load(Ordering::Acquire) == generation
        && shared.armed() == armed
        && (armed || shared.state().playing.is_some())
}

fn run_asio(shared: &Arc<Shared>, armed: bool) -> Result<(), String> {
    let generation = shared.generation.load(Ordering::Acquire);
    let settings = shared.settings();
    let name = settings.asio_driver.clone().or_else(|| crate::asio::drivers().into_iter().next());
    let Some(name) = name else { return Err("no ASIO driver is installed".into()) };
    // The rate is known once the driver runs: the processor made for it then.
    let processor: Arc<Mutex<Option<Processor>>> = Arc::new(Mutex::new(None));
    let made = processor.clone();
    // The copy for Windows (Discord, OBS): what ASIO plays, queued for its own thread.
    let copied: Arc<Mutex<VecDeque<[f32; 2]>>> = Arc::new(Mutex::new(VecDeque::new()));
    let copy_stop = Arc::new(AtomicBool::new(false));
    let copy_thread: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
    let copying = settings.stream_copy;
    let queue = copied.clone();
    let process: crate::asio::Process = Box::new(move |input, out| {
        let mut processor = made.lock().unwrap_or_else(|e| e.into_inner());
        let Some(processor) = processor.as_mut() else { return };
        processor.copying = copying;
        processor.block(input, out);
        if copying {
            let mut queue = queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.extend(processor.copied.iter().copied());
            // Nobody taking it (its output gone): a second at most.
            let most = 192_000;
            if queue.len() > most {
                let over = queue.len() - most;
                queue.drain(..over);
            }
        }
    });
    let started = |running: crate::asio::Running, inputs: Vec<String>, outputs: Vec<String>| {
        *processor.lock().unwrap_or_else(|e| e.into_inner()) = Some(Processor::new(shared.clone(), running.rate));
        let ms = |frames: u32| frames as f32 * 1000.0 / running.rate.max(1) as f32;
        let engine = Engine {
            driver: format!("ASIO: {name}"),
            rate: running.rate,
            buffer: running.buffer,
            input_ms: ms(running.input_latency),
            output_ms: ms(running.output_latency),
        };
        log!(
            "announcements: ASIO {name}, {} Hz, {} frames: in {:.1} ms, out {:.1} ms",
            engine.rate,
            engine.buffer,
            engine.input_ms,
            engine.output_ms
        );
        let mut status = shared.status();
        status.engine = Some(engine);
        status.asio_inputs = inputs;
        status.asio_outputs = outputs;
        drop(status);
        if copying {
            let (queue, stop, output, rate) = (copied.clone(), copy_stop.clone(), settings.stream_output.clone(), running.rate);
            *copy_thread.lock().unwrap_or_else(|e| e.into_inner()) = std::thread::Builder::new()
                .name("westplay-announce-copy".into())
                .spawn(move || {
                    if let Err(e) = copy_to_windows(&queue, &stop, output.as_deref(), rate) {
                        log!("announcements: the copy for Windows: {e}");
                    }
                })
                .ok();
        }
    };
    let keep_going = || going(shared, armed, generation);
    let result =
        crate::asio::run(&name, settings.asio_input, settings.asio_output, settings.buffer, process, &started, &keep_going);
    copy_stop.store(true, Ordering::Release);
    if let Some(thread) = copy_thread.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = thread.join();
    }
    result
}

/// ASIO's sound (at `from_rate`) played on a Windows output too, until `stop`: for Discord sharing the screen (or
/// OBS) to hear it. Not for the player's ears, so a little behind (30 ms queued, never running dry), the two clocks
/// kept together by resampling it a hair faster or slower, as the microphone's under WASAPI.
fn copy_to_windows(
    queue: &Mutex<VecDeque<[f32; 2]>>,
    stop: &AtomicBool,
    output: Option<&str>,
    from_rate: u32,
) -> windows::core::Result<()> {
    // SAFETY: COM calls on interfaces we hold; each buffer used within its GetBuffer/ReleaseBuffer, at its size.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let out = open(&device(&devices, output, eRender)?, 480)?;
        let render: IAudioRenderClient = out.client.GetService()?;
        let size = out.client.GetBufferSize()? as usize;
        let rate = out.format.rate;
        let ahead = ((out.period as usize) * 2).max(rate as usize / 50).min(size);
        let target = from_rate as f32 * 0.03;
        let base = from_rate as f64 / f64::from(rate);
        let (mut at, mut level, mut primed) = (0.0f64, target, false);
        out.client.Start()?;
        log!("announcements: ASIO's sound copied to a Windows output ({rate} Hz)");
        while !stop.load(Ordering::Acquire) {
            if WaitForMultipleObjects(&[out.event], false, 200).0 != WAIT_OBJECT_0.0 {
                continue;
            }
            let padding = out.client.GetCurrentPadding()? as usize;
            let count = ahead.saturating_sub(padding).min(size - padding);
            if count == 0 {
                continue;
            }
            let mut queue = queue.lock().unwrap_or_else(|e| e.into_inner());
            if queue.len() as f32 > target * 4.0 {
                let over = queue.len() - target as usize;
                queue.drain(..over);
            }
            level += (queue.len() as f32 - level) * 0.02;
            let step = base * (1.0 + f64::from(((level - target) / from_rate as f32).clamp(-0.005, 0.005)));
            if !primed && queue.len() as f32 >= target {
                primed = true;
            }
            let data = render.GetBuffer(count as u32)?;
            for frame in 0..count {
                let mut both = [0.0f32; 2];
                if primed && queue.len() >= 2 {
                    let t = at as f32;
                    for (side, value) in both.iter_mut().enumerate() {
                        *value = queue[0][side] * (1.0 - t) + queue[1][side] * t;
                    }
                    at += step;
                    while at >= 1.0 && !queue.is_empty() {
                        queue.pop_front();
                        at -= 1.0;
                    }
                } else {
                    primed = false;
                }
                for channel in 0..out.format.channels {
                    write(data, out.format, frame, channel, if channel < 2 { both[channel] } else { 0.0 });
                }
            }
            render.ReleaseBuffer(count as u32, 0)?;
        }
        Ok(())
    }
}

/// A device's stream format, as far as it's used here.
#[derive(Clone, Copy)]
struct Format {
    channels: usize,
    rate: u32,
    float: bool,
    bits: u16,
}

struct Stream {
    client: IAudioClient,
    format: Format,
    event: HANDLE,
    /// Its period (frames).
    period: u32,
}

impl Drop for Stream {
    fn drop(&mut self) {
        // SAFETY: the stream's own client and event.
        unsafe {
            let _ = self.client.Stop();
            let _ = CloseHandle(self.event);
        }
    }
}

/// The chosen device (by its id), else Windows' default.
fn device(devices: &IMMDeviceEnumerator, id: Option<&str>, flow: EDataFlow) -> windows::core::Result<IMMDevice> {
    // SAFETY: COM calls on the enumerator we hold.
    unsafe {
        if let Some(id) = id
            && let Ok(device) = devices.GetDevice(PCWSTR(HSTRING::from(id).as_ptr()))
        {
            return Ok(device);
        }
        devices.GetDefaultAudioEndpoint(flow, eConsole)
    }
}

/// The device's stream shared with every other app at its own format, at a period as near `buffer` frames as its
/// engine allows (its engine's own already, when another app has set that).
fn open(device: &IMMDevice, buffer: u32) -> windows::core::Result<Stream> {
    // SAFETY: COM calls on interfaces we hold; the mix format freed once used.
    unsafe {
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let mix = client.GetMixFormat()?;
        let wave: WAVEFORMATEX = std::ptr::read_unaligned(mix);
        let (tag, channels, rate, bits) = (wave.wFormatTag, wave.nChannels, wave.nSamplesPerSec, wave.wBitsPerSample);
        let format = Format { channels: usize::from(channels.max(1)), rate, float: tag == 3 || bits == 32, bits };
        let free = || CoTaskMemFree(Some(mix as *const _));
        if let Ok(client3) = client.cast::<IAudioClient3>() {
            let (mut default, mut fundamental, mut min, mut max) = (0, 0, 0, 0);
            if client3.GetSharedModeEnginePeriod(mix, &mut default, &mut fundamental, &mut min, &mut max).is_ok() {
                let fundamental = fundamental.max(1);
                let period = buffer.div_ceil(fundamental).saturating_mul(fundamental).clamp(min, max.max(min));
                if client3.InitializeSharedAudioStream(AUDCLNT_STREAMFLAGS_EVENTCALLBACK, period, mix, None).is_ok() {
                    free();
                    return finish(client, format, period);
                }
                // Another app has the engine at a period of its own: that one (a failed client is spent).
                let mut current: *mut WAVEFORMATEX = std::ptr::null_mut();
                let mut running = 0;
                if client3.GetCurrentSharedModeEnginePeriod(&mut current, &mut running).is_ok() {
                    if !current.is_null() {
                        CoTaskMemFree(Some(current as *const _));
                    }
                    let again: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                    if let Ok(again3) = again.cast::<IAudioClient3>()
                        && again3.InitializeSharedAudioStream(AUDCLNT_STREAMFLAGS_EVENTCALLBACK, running, mix, None).is_ok()
                    {
                        free();
                        return finish(again, format, running);
                    }
                }
            }
        }
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let result = client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK, 0, 0, mix, None);
        free();
        result?;
        let mut default = 0i64;
        client.GetDevicePeriod(Some(&mut default), None)?;
        finish(client, format, (default as u64 * u64::from(rate) / 10_000_000) as u32)
    }
}

/// The microphone for WestPlay alone, at a period as near `buffer` frames as it takes (at `rate` if it can): no
/// Windows mixing or effects between. Refused (another app has the microphone, or Windows doesn't allow it): why.
fn open_exclusive(device: &IMMDevice, buffer: u32, rate: u32) -> Result<Stream, String> {
    // SAFETY: COM calls on interfaces we hold; the formats ours, the mix format freed once read.
    unsafe {
        let probe: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
        let mix = probe.GetMixFormat().map_err(|e| e.to_string())?;
        let wave: WAVEFORMATEX = std::ptr::read_unaligned(mix);
        CoTaskMemFree(Some(mix as *const _));
        let (mut default, mut least) = (0i64, 0i64);
        probe.GetDevicePeriod(Some(&mut default), Some(&mut least)).map_err(|e| e.to_string())?;
        let mut tried = Vec::new();
        for rate in [rate, wave.nSamplesPerSec, 48_000, 44_100] {
            for channels in [wave.nChannels, 1, 2] {
                for (tag, bits) in [(1u16, 16u16), (3, 32), (1, 32)] {
                    if tried.contains(&(rate, channels, tag, bits)) {
                        continue;
                    }
                    tried.push((rate, channels, tag, bits));
                    let block = channels * bits / 8;
                    let format = WAVEFORMATEX {
                        wFormatTag: tag,
                        nChannels: channels,
                        nSamplesPerSec: rate,
                        nAvgBytesPerSec: rate * u32::from(block),
                        nBlockAlign: block,
                        wBitsPerSample: bits,
                        cbSize: 0,
                    };
                    if probe.IsFormatSupported(AUDCLNT_SHAREMODE_EXCLUSIVE, &format, None) != S_OK {
                        continue;
                    }
                    let ours = Format { channels: usize::from(channels), rate, float: tag == 3, bits };
                    let period = ((f64::from(buffer) * 10_000_000.0 / f64::from(rate)) as i64).max(least);
                    let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
                    let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                    let client = match client.Initialize(AUDCLNT_SHAREMODE_EXCLUSIVE, flags, period, period, &format, None) {
                        Ok(()) => client,
                        // The period as many whole frames as the device takes.
                        Err(e) if e.code() == AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED => {
                            let frames = client.GetBufferSize().map_err(|e| e.to_string())?;
                            let aligned = (10_000_000.0 * f64::from(frames) / f64::from(rate) + 0.5) as i64;
                            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
                            client
                                .Initialize(AUDCLNT_SHAREMODE_EXCLUSIVE, flags, aligned, aligned, &format, None)
                                .map_err(|e| format!("the microphone can't be WestPlay's alone: {e}"))?;
                            client
                        }
                        Err(e) => return Err(format!("the microphone can't be WestPlay's alone: {e}")),
                    };
                    let period = client.GetBufferSize().map_err(|e| e.to_string())?;
                    return finish(client, ours, period).map_err(|e| e.to_string());
                }
            }
        }
        Err("the microphone takes none of WestPlay's formats for itself alone".into())
    }
}

fn finish(client: IAudioClient, format: Format, period: u32) -> windows::core::Result<Stream> {
    // SAFETY: an event of our own, for the client we hold.
    unsafe {
        let event = CreateEventW(None, false, false, None)?;
        let stream = Stream { client, format, event, period: period.max(1) };
        stream.client.SetEventHandle(event)?;
        Ok(stream)
    }
}

/// WASAPI: the speakers (shared: the game plays on them too), and the microphone (shared, or WestPlay's alone with
/// `exclusive`), each at a period as near the buffer as it allows, on a thread Windows schedules as audio's.
///
/// The microphone and the speakers each keep their own clock: what the microphone brings waits in a queue for the
/// speakers, kept as short as never runs dry by resampling it a hair faster or slower as the queue grows or
/// shrinks (no sample dropped or repeated: no crackle).
fn run_wasapi(shared: &Arc<Shared>, armed: bool, exclusive: bool) -> windows::core::Result<()> {
    let generation = shared.generation.load(Ordering::Acquire);
    let settings = shared.settings();
    let buffer = settings.buffer.clamp(16, 4096);
    // SAFETY: Windows' audio scheduling for this thread, reverted on the way out.
    let task = unsafe {
        let mut index = 0;
        AvSetMmThreadCharacteristicsW(windows::core::w!("Pro Audio"), &mut index).ok()
    };
    let result = (|| {
        // SAFETY: COM calls on interfaces we hold; each buffer used within its GetBuffer/ReleaseBuffer, at its size.
        unsafe {
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let out = open(&device(&devices, settings.output.as_deref(), eRender)?, buffer)?;
            let mic = if armed {
                let found = device(&devices, settings.input.as_deref(), eCapture)?;
                Some(if exclusive {
                    match open_exclusive(&found, buffer, out.format.rate) {
                        Ok(stream) => stream,
                        Err(e) => {
                            shared.status().error = Some(e.clone());
                            log!("announcements: {e}; shared instead");
                            open(&found, buffer)?
                        }
                    }
                } else {
                    open(&found, buffer)?
                })
            } else {
                None
            };
            let capture: Option<IAudioCaptureClient> = match &mic {
                Some(mic) => Some(mic.client.GetService()?),
                None => None,
            };
            let render: IAudioRenderClient = out.client.GetService()?;
            let size = out.client.GetBufferSize()? as usize;
            let rate = out.format.rate;
            let pass = out.period as usize;
            // The speakers' buffer kept a pass and the player's buffer ahead.
            let ahead = (pass + buffer as usize).min(size);
            // The microphone's sound queued for the speakers: a packet of its and a pass of theirs.
            let packet = mic.as_ref().map_or(0, |mic| (mic.period as u64 * u64::from(rate) / u64::from(mic.format.rate)) as usize);
            let queue_target = (packet + pass) as f32;
            let base = mic.as_ref().map_or(1.0, |mic| mic.format.rate as f32 / rate as f32);
            let ms = |frames: f32| frames * 1000.0 / rate as f32;
            let stream_ms = |stream: &Stream| stream.client.GetStreamLatency().map_or(0.0, |hns| hns as f32 / 10_000.0);
            let engine = Engine {
                driver: if mic.as_ref().is_some() && exclusive { "WASAPI exclusive".into() } else { "WASAPI".into() },
                rate,
                buffer,
                input_ms: mic.as_ref().map_or(0.0, |mic| ms(packet as f32) + ms(queue_target) + stream_ms(mic)),
                output_ms: ms(ahead as f32) + stream_ms(&out),
            };
            log!(
                "announcements: {}, {} Hz, buffer {}: the speakers {:.1} ms a pass, the microphone {}, in {:.1} ms, out {:.1} ms",
                engine.driver,
                rate,
                buffer,
                ms(pass as f32),
                mic.as_ref().map_or("off".to_owned(), |mic| format!("{:.1} ms a packet", mic.period as f32 * 1000.0 / mic.format.rate as f32)),
                engine.input_ms,
                engine.output_ms
            );
            shared.status().engine = Some(engine);
            let mut processor = Processor::new(shared.clone(), rate);
            let mut resampler = Resampler::new(base);
            let mut queue: VecDeque<f32> = VecDeque::new();
            let mut fresh = Vec::new();
            let mut heard = Vec::new();
            let mut played = Vec::new();
            let mut level = queue_target;
            let mut primed = false;
            if let Some(mic) = &mic {
                mic.client.Start()?;
            }
            out.client.Start()?;
            let events: Vec<HANDLE> = std::iter::once(out.event).chain(mic.as_ref().map(|mic| mic.event)).collect();
            while going(shared, armed, generation) {
                let woken: WAIT_EVENT = WaitForMultipleObjects(&events, false, 200);
                if woken.0 >= WAIT_OBJECT_0.0 + events.len() as u32 {
                    continue;
                }
                if let (Some(mic), Some(capture)) = (&mic, &capture) {
                    loop {
                        if capture.GetNextPacketSize()? == 0 {
                            break;
                        }
                        let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
                        capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                        let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                        fresh.clear();
                        for frame in 0..frames as usize {
                            let sample = if silent { 0.0 } else { read(data, mic.format, frame) };
                            resampler.push(sample, &mut fresh);
                        }
                        capture.ReleaseBuffer(frames)?;
                        queue.extend(fresh.iter());
                    }
                    // Far behind (the thread held up): back to the target at once.
                    let most = queue_target as usize + 4 * (packet + pass);
                    if queue.len() > most {
                        queue.drain(..queue.len() - queue_target as usize);
                    }
                }
                let padding = out.client.GetCurrentPadding()? as usize;
                let count = ahead.saturating_sub(padding).min(size - padding);
                if count == 0 {
                    continue;
                }
                if mic.is_some() {
                    // The queue's level as the speakers take from it, its resampling nudged to keep it at the
                    // target: a second's worth of the difference made up over a second, at most 0.5% either way.
                    level += (queue.len() as f32 - level) * 0.02;
                    let off = ((level - queue_target) / rate as f32).clamp(-0.005, 0.005);
                    resampler.step = base * (1.0 + off);
                    if !primed && queue.len() as f32 >= queue_target {
                        primed = true;
                    }
                }
                heard.clear();
                if primed {
                    let take = count.min(queue.len());
                    heard.extend(queue.drain(..take));
                    if take < count {
                        // Run dry: waits for the target again.
                        primed = false;
                    }
                }
                heard.resize(count, 0.0);
                played.clear();
                played.resize(count, [0.0f32; 2]);
                processor.block(&heard, &mut played);
                let data = render.GetBuffer(count as u32)?;
                for (frame, [left, right]) in played.iter().enumerate() {
                    for channel in 0..out.format.channels {
                        let value = match channel {
                            0 => *left,
                            1 => *right,
                            _ => 0.0,
                        };
                        write(data, out.format, frame, channel, value);
                    }
                }
                render.ReleaseBuffer(count as u32, 0)?;
            }
            Ok(())
        }
    })();
    if let Some(task) = task {
        // SAFETY: the task this thread was given.
        let _ = unsafe { AvRevertMmThreadCharacteristics(task) };
    }
    result
}

/// A frame's first channel (the microphone's voice).
unsafe fn read(data: *mut u8, format: Format, frame: usize) -> f32 {
    let at = frame * format.channels;
    // SAFETY: inside the buffer the caller has (`frame` under its frames).
    unsafe {
        if format.float {
            *(data as *const f32).add(at)
        } else if format.bits == 16 {
            f32::from(*(data as *const i16).add(at)) / 32768.0
        } else {
            *(data as *const i32).add(at) as f32 / 2_147_483_648.0
        }
    }
}

unsafe fn write(data: *mut u8, format: Format, frame: usize, channel: usize, value: f32) {
    let at = frame * format.channels + channel;
    let value = value.clamp(-1.0, 1.0);
    // SAFETY: inside the buffer the caller has (`frame` under its frames).
    unsafe {
        if format.float {
            *(data as *mut f32).add(at) = value;
        } else if format.bits == 16 {
            *(data as *mut i16).add(at) = (value * 32767.0) as i16;
        } else {
            *(data as *mut i32).add(at) = (f64::from(value) * 2_147_483_647.0) as i32;
        }
    }
}

/// From the microphone's rate to the speakers', each speaker sample between two of the microphone's; `step` the
/// microphone's samples per speaker sample (nudged to keep the queue level).
struct Resampler {
    step: f32,
    at: f32,
    last: f32,
}

impl Resampler {
    fn new(step: f32) -> Self {
        Self { step, at: 0.0, last: 0.0 }
    }

    fn push(&mut self, sample: f32, out: &mut Vec<f32>) {
        while self.at < 1.0 {
            out.push(self.last + (sample - self.last) * self.at);
            self.at += self.step;
        }
        self.at -= 1.0;
        self.last = sample;
    }
}

/// The recording made so far, kept in its slot (and its file).
fn keep_recording(dir: &Path, state: &mut State) {
    let Some((slot, samples, rate)) = state.recording.take() else { return };
    if rate == 0 || samples.len() < (rate / 4) as usize {
        return;
    }
    let recording = Recording { rate, samples: Arc::new(samples) };
    let (file, saved) = (slot_file(dir, slot), recording.clone());
    std::thread::spawn(move || {
        if let Err(e) = save(&file, &saved) {
            log!("announcements: cannot save {}: {e}", file.display());
        }
    });
    log!("announcements: slot {} recorded", slot + 1);
    state.slots[slot] = Some(recording);
}

fn slot_file(dir: &Path, slot: usize) -> PathBuf {
    dir.join(format!("slot{}.wav", slot + 1))
}

/// A 16-bit mono WAV.
fn save(file: &Path, recording: &Recording) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let data = (recording.samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&recording.rate.to_le_bytes());
    bytes.extend_from_slice(&(recording.rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for sample in recording.samples.iter() {
        bytes.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(file, bytes)
}

/// A WAV as [`save`] writes them.
fn load(file: &Path) -> Option<Recording> {
    let bytes = std::fs::read(file).ok()?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let (mut at, mut rate, mut channels) = (12, 0u32, 1usize);
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
        if id == b"fmt " && body.len() >= 16 {
            channels = usize::from(u16::from_le_bytes([body[2], body[3]]).max(1));
            rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
            if u16::from_le_bytes([body[14], body[15]]) != 16 {
                return None;
            }
        } else if id == b"data" && rate > 0 {
            let samples = body
                .chunks_exact(2 * channels)
                .map(|frame| f32::from(i16::from_le_bytes([frame[0], frame[1]])) / 32768.0)
                .collect();
            return Some(Recording { rate, samples: Arc::new(samples) });
        }
        at += 8 + size + (size & 1);
    }
    None
}

/// A biquad (RBJ's cookbook).
#[derive(Clone, Copy)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    fn new(rate: f32, frequency: f32, high: bool) -> Self {
        let w = std::f32::consts::TAU * frequency / rate;
        let alpha = w.sin() / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let cos = w.cos();
        let a0 = 1.0 + alpha;
        let b = if high {
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0]
        } else {
            [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0]
        };
        Self { b: b.map(|v| v / a0), a: [-2.0 * cos / a0, (1.0 - alpha) / a0], x: [0.0; 2], y: [0.0; 2] }
    }

    /// A bell at `frequency`, `db` up (RBJ's peaking EQ).
    fn bell(rate: f32, frequency: f32, q: f32, db: f32) -> Self {
        let w = std::f32::consts::TAU * frequency / rate;
        let alpha = w.sin() / (2.0 * q);
        let a = 10f32.powf(db / 40.0);
        let cos = w.cos();
        let a0 = 1.0 + alpha / a;
        Self {
            b: [(1.0 + alpha * a) / a0, -2.0 * cos / a0, (1.0 - alpha * a) / a0],
            a: [-2.0 * cos / a0, (1.0 - alpha / a) / a0],
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }

    fn step(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// The voice as the bus's PA plays it, at the speakers' rate.
struct Voice {
    rate: f32,
    filters: [Biquad; 4],
    gain: f32,
    /// The speakers' quality the filters are for (0 old, 100 new), and what it makes of the voice: how hard the
    /// units are driven, and their hiss and crackle.
    quality: Option<u32>,
    drive: f32,
    hiss: f32,
    crackle: f32,
    noise: u32,
    /// The voice's last 150 ms, for the further units.
    history: Vec<f32>,
    written: usize,
    taps: Vec<(usize, f32, usize)>,
    set: Option<Vec<Tap>>,
}

impl Voice {
    fn new(rate: f32) -> Self {
        Self {
            rate,
            filters: [Biquad::new(rate, 220.0, true); 4],
            gain: 0.8,
            quality: None,
            drive: 1.6,
            hiss: 0.0,
            crackle: 0.0,
            noise: 0x1234_5678,
            history: vec![0.0; (rate * 0.15) as usize + 1],
            written: 0,
            taps: Vec::new(),
            set: None,
        }
    }

    /// From worn-out old speakers (0: a narrow, honky band, driven into distortion, hissing and crackling) to new
    /// ones (100: a wide, clean band), every step between.
    fn set_quality(&mut self, quality: u32) {
        if self.quality == Some(quality) {
            return;
        }
        self.quality = Some(quality);
        let new = quality.min(100) as f32 / 100.0;
        let old = 1.0 - new;
        // The band: 450 Hz to 2.8 kHz worn out, 90 Hz to 14 kHz new (by octaves between).
        let low = 450.0 * (90.0f32 / 450.0).powf(new);
        let high = 2800.0 * (14_000.0f32 / 2800.0).powf(new);
        self.filters = [
            Biquad::new(self.rate, low, true),
            Biquad::new(self.rate, low, true),
            Biquad::new(self.rate, high, false),
            // A tired cone's honk.
            Biquad::bell(self.rate, 1600.0, 1.4, 9.0 * old),
        ];
        self.drive = 1.0 + 5.0 * old * old + 0.6 * old;
        self.hiss = 0.006 * old * old;
        // Crackle only from really worn ones: at most about once a second.
        self.crackle = 0.00002 * ((old - 0.6).max(0.0) / 0.4).powi(2);
    }

    /// The next of a cheap noise (xorshift), -1 to 1.
    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    fn set_taps(&mut self, taps: &[Tap]) {
        if self.set.as_deref() == Some(taps) {
            return;
        }
        self.set = Some(taps.to_vec());
        // No coach's units known: one speaker each side, at once.
        let taps = if taps.is_empty() {
            vec![Tap { delay: 0.0, gain: 1.0, side: 0 }, Tap { delay: 0.0, gain: 1.0, side: 1 }]
        } else {
            taps.to_vec()
        };
        // Each side as loud, all its units together, as one speaker.
        let mut sums = [0.0f32; 2];
        for tap in &taps {
            sums[tap.side.min(1)] += tap.gain;
        }
        let longest = self.history.len() - 1;
        self.taps = taps
            .iter()
            .map(|tap| {
                let side = tap.side.min(1);
                let delay = ((tap.delay.max(0.0) / 1000.0 * self.rate) as usize).min(longest);
                (delay, 1.5 * tap.gain / sums[side].max(1.0), side)
            })
            .collect();
    }

    /// The voice's next sample; `active`: the PA heard (its hiss only then).
    fn play(&mut self, x: f32, active: bool) -> [f32; 2] {
        let mut v = x * 2.0;
        for filter in &mut self.filters {
            v = filter.step(v);
        }
        // A PA's units, driven as hard as they're worn; an old one's hiss, and now and then its crackle.
        let mut v = (v * self.drive).tanh() / self.drive.tanh().max(0.5);
        if self.hiss > 0.0 && active {
            v += self.noise() * self.hiss;
            if self.noise() > 1.0 - self.crackle * 2.0 {
                v += self.noise() * 0.3;
            }
        }
        let v = v * self.gain;
        let len = self.history.len();
        self.written = (self.written + 1) % len;
        self.history[self.written] = v;
        let mut both = [0.0; 2];
        for &(delay, gain, side) in &self.taps {
            both[side] += self.history[(self.written + len - delay) % len] * gain;
        }
        both
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_saved_loads_back() {
        let dir = std::env::temp_dir().join(format!("westplay-announce-{}", std::process::id()));
        let file = slot_file(&dir, 0);
        let recording = Recording { rate: 48_000, samples: Arc::new(vec![0.0, 0.5, -0.5, 0.25]) };
        save(&file, &recording).unwrap();
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.rate, 48_000);
        assert_eq!(loaded.samples.len(), 4);
        assert!((loaded.samples[1] - 0.5).abs() < 0.001);
        let _ = std::fs::remove_dir_all(dir);
    }
}
