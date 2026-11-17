//! A player's picture for the iPad (`remote`): what the iPad's page can't draw itself (YouTube's and YouTube Music's
//! video, Safari's pages, which play in WestPlay's own browsers on the PC) captured from that browser's visual with
//! Windows Graphics Capture, at most [`FPS`] a second, made a JPEG off the host's thread and sent to the iPad, which
//! shows it under its page where the page leaves room for it, as the truck's frame does.
//!
//! A frame is copied to a readable texture as it's taken and read a turn later without waiting for the GPU, so the
//! host's loop never waits on it; one being encoded when the next is ready is skipped (the newest goes).

use std::cell::{Cell, RefCell};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::UI::Composition::Visual;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAP_FLAG_DO_NOT_WAIT, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::core::{IInspectable, Interface, Result};

use crate::gpu::Gpu;
use crate::remote::Remote;

/// Pictures a second to the iPad.
pub const FPS: u64 = 60;
const INTERVAL: Duration = Duration::from_millis(1000 / FPS);
const QUALITY: u8 = 80;

/// Which player a picture is (its message's first byte, as the iPad's page reads it).
pub const YOUTUBE: u8 = 1;
pub const YOUTUBE_MUSIC: u8 = 2;
pub const SAFARI: u8 = 3;

/// A frame read, for the encoder: its player, size and BGRA rows.
pub struct Picture {
    tag: u8,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// The encoder's thread: JPEGs of the pictures, sent to the iPad. Ends as the sender goes.
pub fn encoder(remote: Arc<Remote>) -> SyncSender<Picture> {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<Picture>(1);
    let _ = std::thread::Builder::new().name("westplay-ipad-pictures".into()).spawn(move || encode(&receiver, &remote));
    sender
}

fn encode(receiver: &Receiver<Picture>, remote: &Remote) {
    for picture in receiver {
        let mut rgb = Vec::with_capacity((picture.width * picture.height * 3) as usize);
        for pixel in picture.pixels.chunks_exact(4) {
            rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        let mut out = vec![picture.tag];
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, QUALITY);
        if encoder.encode(&rgb, picture.width, picture.height, image::ExtendedColorType::Rgb8).is_ok() {
            remote.post_binary(out);
        }
    }
}

pub struct Mirror {
    pub tag: u8,
    /// What's mirrored (a Safari tab's id; 0 for the others): another is another mirror.
    pub of: u64,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    size: Cell<SizeInt32>,
    latest: Arc<Mutex<Option<Direct3D11CaptureFrame>>>,
    /// The readable copy and the size copied into it, waiting to be read.
    staging: RefCell<Option<(ID3D11Texture2D, u32, u32)>>,
    waiting: Cell<Option<(u32, u32)>>,
    taken: Cell<Option<Instant>>,
}

impl Drop for Mirror {
    fn drop(&mut self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

impl Mirror {
    pub fn start(gpu: &Gpu, visual: &Visual, tag: u8, of: u64) -> Result<Self> {
        let item = GraphicsCaptureItem::CreateFromVisual(visual)?;
        let size = item.Size()?;
        let size = SizeInt32 { Width: size.Width.max(1), Height: size.Height.max(1) };
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &gpu.winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        let _ = session.SetIsCursorCaptureEnabled(false);
        let latest = Arc::new(Mutex::new(None));
        pool.FrameArrived(&TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
            let latest = latest.clone();
            move |pool, _| {
                let Some(pool) = pool.as_ref() else { return Ok(()) };
                let frame = pool.TryGetNextFrame()?;
                // The frame it replaces goes back to the pool once the lock is let go.
                let replaced = latest.lock().unwrap_or_else(|e| e.into_inner()).replace(frame);
                drop(replaced);
                Ok(())
            }
        }))?;
        session.StartCapture()?;
        Ok(Self {
            tag,
            of,
            pool,
            session,
            size: Cell::new(size),
            latest,
            staging: RefCell::new(None),
            waiting: Cell::new(None),
            taken: Cell::new(None),
        })
    }

    /// Once a turn: the copy made last turn read (if the GPU is done) and handed to the encoder; then, when one is
    /// due, the newest frame copied.
    pub fn step(&self, gpu: &Gpu, encoder: &SyncSender<Picture>) {
        if let Some((width, height)) = self.waiting.get()
            && let Some((staging, _, _)) = &*self.staging.borrow()
        {
            // SAFETY: the readable texture is ours, mapped for reading and unmapped after; rows are read within the
            // mapped pitch and the size copied into it.
            unsafe {
                let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                if gpu
                    .context
                    .Map(staging, 0, D3D11_MAP_READ, D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32, Some(&mut mapped))
                    .is_err()
                {
                    return;
                }
                let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height as usize {
                    pixels.extend_from_slice(std::slice::from_raw_parts(
                        (mapped.pData as *const u8).add(row * mapped.RowPitch as usize),
                        width as usize * 4,
                    ));
                }
                gpu.context.Unmap(staging, 0);
                self.waiting.set(None);
                match encoder.try_send(Picture { tag: self.tag, width, height, pixels }) {
                    Ok(()) | Err(TrySendError::Full(_)) => {}
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
        }
        if self.taken.get().is_some_and(|at| at.elapsed() < INTERVAL) {
            return;
        }
        let Some(frame) = self.latest.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
        self.taken.set(Some(Instant::now()));
        let _ = self.copy(gpu, &frame);
    }

    fn copy(&self, gpu: &Gpu, frame: &Direct3D11CaptureFrame) -> Result<()> {
        let content = frame.ContentSize()?;
        // The browser laid out at another size: frames of that size from now on.
        if content.Width != self.size.get().Width || content.Height != self.size.get().Height {
            let size = SizeInt32 { Width: content.Width.max(1), Height: content.Height.max(1) };
            self.pool.Recreate(&gpu.winrt_device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
            self.size.set(size);
        }
        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
        // SAFETY: the frame's surface is alive while `frame` is.
        let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: a live texture; the description is a local.
        unsafe { texture.GetDesc(&mut desc) };
        let width = (content.Width.max(1) as u32).min(desc.Width);
        let height = (content.Height.max(1) as u32).min(desc.Height);
        let mut staging = self.staging.borrow_mut();
        if !staging.as_ref().is_some_and(|(_, w, h)| *w == desc.Width && *h == desc.Height) {
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: desc.Width,
                Height: desc.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut made = None;
            // SAFETY: the device is alive; the description is a local.
            unsafe { gpu.device.CreateTexture2D(&staging_desc, None, Some(&mut made))? };
            *staging = made.map(|texture| (texture, desc.Width, desc.Height));
        }
        let Some((target, _, _)) = &*staging else { return Ok(()) };
        let region = D3D11_BOX { left: 0, top: 0, front: 0, right: width, bottom: height, back: 1 };
        // SAFETY: both textures are alive on this device; the region lies within both; the copy isn't mapped.
        unsafe {
            gpu.context.CopySubresourceRegion(target, 0, 0, 0, 0, &texture, 0, Some(&region));
            gpu.context.Flush();
        }
        self.waiting.set(Some((width, height)));
        Ok(())
    }
}
