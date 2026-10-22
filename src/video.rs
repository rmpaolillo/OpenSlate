//! Il video della pagina che si riscrive da sola (MP4, H.264), con il
//! codificatore del sistema, così il programma non si appesantisce e i
//! diritti dell'H.264 sono già pagati da chi lo fornisce:
//! - macOS: AVFoundation (un piccolo aiutante in Swift, `src/swift/video_mp4.swift`);
//! - Windows: Media Foundation;
//! - Linux: `ffmpeg`, se è installato con un codificatore H.264, altrimenti
//!   la libreria OpenH264 di Cisco, scaricata dal sito di Cisco al primo uso
//!   (con il permesso dell'utente) e controllata con la sua impronta SHA-256.
//!
//! I fotogrammi sono quelli della GIF animata (`formats`): i tratti nell'ordine
//! in cui sono stati scritti, in ~6 s, poi la pagina ferma 2 s.

use crate::scene::Page;
use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;

/// Fotogrammi al secondo.
pub const FPS: u32 = 30;
/// Secondi per scrivere la pagina e per tenerla ferma alla fine.
const DRAW_S: f32 = 6.0;
const HOLD_S: f32 = 2.0;
/// Pixel del lato lungo.
const LONG: u32 = 1080;

/// I fotogrammi del video (RGBA, righe dall'alto), uno alla volta.
pub struct Frames<'a> {
    page: &'a Page,
    /// Larghezza e altezza pari (l'H.264 le vuole così).
    pub w: usize,
    pub h: usize,
    full: (usize, usize),
    last: Vec<u8>,
}

impl<'a> Frames<'a> {
    pub fn new(page: &'a Page) -> Result<Frames<'a>> {
        if page.strokes.iter().all(|s| s.pts.is_empty()) {
            bail!("pagina vuota");
        }
        let (w0, h0, last) = crate::export::page_to_rgba(page, LONG)?;
        let (w, h) = (w0 & !1, h0 & !1);
        let mut f = Frames { page, w, h, full: (w0, h0), last: Vec::new() };
        f.last = f.crop(last);
        Ok(f)
    }

    pub fn count() -> usize {
        ((DRAW_S + HOLD_S) * FPS as f32).round() as usize
    }

    fn crop(&self, rgba: Vec<u8>) -> Vec<u8> {
        let (w0, h0) = self.full;
        if (w0, h0) == (self.w, self.h) {
            return rgba;
        }
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for y in 0..self.h {
            out.extend_from_slice(&rgba[y * w0 * 4..y * w0 * 4 + self.w * 4]);
        }
        out
    }

    /// Chiama `f` per ogni fotogramma, in ordine.
    pub fn each(&self, mut f: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
        let tl = crate::formats::timeline(self.page);
        let draw = (DRAW_S * FPS as f32).round() as usize;
        for k in 0..draw {
            let t = k as f32 / draw as f32;
            let page = crate::formats::page_at(self.page, &tl, t);
            let (_, _, rgba) = crate::export::page_to_rgba(&page, LONG)?;
            f(&self.crop(rgba))?;
        }
        for _ in draw..Self::count() {
            f(&self.last)?;
        }
        Ok(())
    }
}

/// RGBA → BGRA (il formato dei codificatori di macOS e Windows).
#[cfg_attr(target_os = "linux", allow(dead_code))]
fn to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut v = rgba.to_vec();
    for px in v.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    v
}

/// Il video della pagina in `path` (MP4).
pub fn write_mp4(page: &Page, path: &Path) -> Result<()> {
    let frames = Frames::new(page)?;
    encode(&frames, path)?;
    if std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) == 0 {
        bail!("il video non è stato scritto");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// macOS: AVFoundation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn encode(fr: &Frames, out: &Path) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let bin = crate::swift::helper("video_mp4", include_str!("swift/video_mp4.swift"))?;
    let mut child = Command::new(bin)
        .arg(fr.w.to_string())
        .arg(fr.h.to_string())
        .arg(FPS.to_string())
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("aiutante video")?;
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("aiutante video: niente stdin"))?;
    let sent = fr.each(|rgba| stdin.write_all(&to_bgra(rgba)).map_err(Into::into));
    drop(stdin);
    let o = child.wait_with_output()?;
    if !o.status.success() {
        bail!("video (AVFoundation): {}", String::from_utf8_lossy(&o.stderr).trim());
    }
    sent
}

// ---------------------------------------------------------------------------
// Windows: Media Foundation
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn encode(fr: &Frames, out: &Path) -> Result<()> {
    mf::encode(fr, out)
}

#[cfg(windows)]
mod mf {
    use super::{to_bgra, Frames, FPS};
    use anyhow::{anyhow, Result};
    use std::path::Path;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Media::MediaFoundation::*;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    /// Spegne Media Foundation (e COM) anche se qualcosa va storto.
    struct Session(bool);
    impl Drop for Session {
        fn drop(&mut self) {
            unsafe {
                let _ = MFShutdown();
                if self.0 {
                    CoUninitialize();
                }
            }
        }
    }

    pub fn encode(fr: &Frames, out: &Path) -> Result<()> {
        let (w, h) = (fr.w as u32, fr.h as u32);
        let e = |what: &str, err: windows::core::Error| anyhow!("video (Media Foundation), {what}: {err}");
        unsafe {
            let com = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(|x| e("avvio", x))?;
            let _session = Session(com);
            let url = HSTRING::from(out.as_os_str());
            let writer: IMFSinkWriter = MFCreateSinkWriterFromURL(PCWSTR(url.as_ptr()), None, None).map_err(|x| e("file", x))?;
            let size = ((w as u64) << 32) | h as u64;
            let rate = ((FPS as u64) << 32) | 1;
            let aspect = (1u64 << 32) | 1;
            // in uscita: H.264
            let outt: IMFMediaType = MFCreateMediaType().map_err(|x| e("tipo", x))?;
            outt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|x| e("tipo", x))?;
            outt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).map_err(|x| e("tipo", x))?;
            outt.SetUINT32(&MF_MT_AVG_BITRATE, (w * h * 3).max(1_000_000)).map_err(|x| e("tipo", x))?;
            outt.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(|x| e("tipo", x))?;
            outt.SetUINT64(&MF_MT_FRAME_SIZE, size).map_err(|x| e("tipo", x))?;
            outt.SetUINT64(&MF_MT_FRAME_RATE, rate).map_err(|x| e("tipo", x))?;
            outt.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, aspect).map_err(|x| e("tipo", x))?;
            let stream = writer.AddStream(&outt).map_err(|x| e("flusso", x))?;
            // in entrata: BGRA a 32 bit, righe dall'alto (passo positivo)
            let int: IMFMediaType = MFCreateMediaType().map_err(|x| e("tipo", x))?;
            int.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|x| e("tipo", x))?;
            int.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(|x| e("tipo", x))?;
            int.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).map_err(|x| e("tipo", x))?;
            int.SetUINT32(&MF_MT_DEFAULT_STRIDE, w * 4).map_err(|x| e("tipo", x))?;
            int.SetUINT64(&MF_MT_FRAME_SIZE, size).map_err(|x| e("tipo", x))?;
            int.SetUINT64(&MF_MT_FRAME_RATE, rate).map_err(|x| e("tipo", x))?;
            int.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, aspect).map_err(|x| e("tipo", x))?;
            writer.SetInputMediaType(stream, &int, None).map_err(|x| e("ingresso", x))?;
            writer.BeginWriting().map_err(|x| e("inizio", x))?;
            // durata di un fotogramma in unità da 100 ns
            let dur = 10_000_000i64 / FPS as i64;
            let len = w * h * 4;
            let mut k: i64 = 0;
            fr.each(|rgba| {
                let bgra = to_bgra(rgba);
                let buf: IMFMediaBuffer = MFCreateMemoryBuffer(len).map_err(|x| e("memoria", x))?;
                let mut p: *mut u8 = std::ptr::null_mut();
                buf.Lock(&mut p, None, None).map_err(|x| e("memoria", x))?;
                std::ptr::copy_nonoverlapping(bgra.as_ptr(), p, len as usize);
                buf.Unlock().map_err(|x| e("memoria", x))?;
                buf.SetCurrentLength(len).map_err(|x| e("memoria", x))?;
                let sample: IMFSample = MFCreateSample().map_err(|x| e("fotogramma", x))?;
                sample.AddBuffer(&buf).map_err(|x| e("fotogramma", x))?;
                sample.SetSampleTime(k * dur).map_err(|x| e("fotogramma", x))?;
                sample.SetSampleDuration(dur).map_err(|x| e("fotogramma", x))?;
                writer.WriteSample(stream, &sample).map_err(|x| e("fotogramma", x))?;
                k += 1;
                Ok(())
            })?;
            writer.Finalize().map_err(|x| e("chiusura", x))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Linux: ffmpeg o OpenH264 di Cisco
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn encode(fr: &Frames, out: &Path) -> Result<()> {
    if let Some(codec) = ffmpeg_h264() {
        return ffmpeg(fr, out, codec);
    }
    let lib = cisco::lib_path();
    if !lib.exists() {
        bail!("per il video serve il codificatore OpenH264 di Cisco (o ffmpeg): riprova e accetta di scaricarlo");
    }
    let api = openh264::OpenH264API::from_blob_path(&lib).map_err(|e| anyhow!("OpenH264 di Cisco: {e}"))?;
    h264::encode(api, fr, out)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn encode(_fr: &Frames, _out: &Path) -> Result<()> {
    bail!("il video MP4 non c'è su questo sistema")
}

/// Il codificatore H.264 di ffmpeg, se ffmpeg c'è e ne ha uno.
#[cfg(target_os = "linux")]
fn ffmpeg_h264() -> Option<&'static str> {
    let o = std::process::Command::new("ffmpeg").args(["-hide_banner", "-encoders"]).output().ok()?;
    let list = String::from_utf8_lossy(&o.stdout).to_string();
    ["libx264", "libopenh264"].into_iter().find(|c| list.split_whitespace().any(|w| w == *c))
}

#[cfg(target_os = "linux")]
fn ffmpeg(fr: &Frames, out: &Path, codec: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba"])
        .arg("-s")
        .arg(format!("{}x{}", fr.w, fr.h))
        .arg("-r")
        .arg(FPS.to_string())
        .args(["-i", "-", "-c:v", codec, "-pix_fmt", "yuv420p"]);
    if codec == "libx264" {
        cmd.args(["-preset", "medium", "-crf", "20", "-tune", "stillimage"]);
    } else {
        cmd.args(["-b:v", "3M"]);
    }
    cmd.args(["-movflags", "+faststart"]).arg(out);
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().context("ffmpeg")?;
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("ffmpeg: niente stdin"))?;
    let sent = fr.each(|rgba| stdin.write_all(rgba).map_err(Into::into));
    drop(stdin);
    let o = child.wait_with_output()?;
    if !o.status.success() {
        bail!("ffmpeg: {}", String::from_utf8_lossy(&o.stderr).trim());
    }
    sent
}

/// Su Linux: va chiesto se scaricare il codificatore di Cisco (non c'è
/// ffmpeg con l'H.264 e la libreria non è ancora stata scaricata).
pub fn needs_codec_download() -> bool {
    #[cfg(target_os = "linux")]
    {
        ffmpeg_h264().is_none() && cisco::FILE.is_some() && !cisco::lib_path().exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Scarica il codificatore di Cisco (solo Linux).
pub fn download_codec() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        cisco::download()
    }
    #[cfg(not(target_os = "linux"))]
    {
        bail!("serve solo su Linux")
    }
}

/// L'avviso di Cisco (va mostrato a chi usa la loro libreria).
pub const CISCO_NOTICE: &str = "OpenH264 Video Codec provided by Cisco Systems, Inc.";

#[cfg(target_os = "linux")]
pub mod cisco {
    use anyhow::{anyhow, bail, Context, Result};
    use std::path::PathBuf;

    /// La libreria per questo processore (quelle che `openh264` sa usare: 2.6.0).
    pub const FILE: Option<&str> = if cfg!(target_arch = "x86_64") {
        Some("libopenh264-2.6.0-linux64.8.so")
    } else if cfg!(target_arch = "aarch64") {
        Some("libopenh264-2.6.0-linux-arm64.8.so")
    } else {
        None
    };

    /// Dove sta, nella cartella dei dati.
    pub fn lib_path() -> PathBuf {
        PathBuf::from("codecs").join(FILE.unwrap_or("libopenh264.so"))
    }

    /// La scarica da Cisco (come Firefox), la scompatta e la tiene solo se la
    /// sua impronta è una di quelle conosciute.
    pub fn download() -> Result<()> {
        let Some(file) = FILE else { bail!("OpenH264 di Cisco: non c'è per questo processore") };
        let mut last = None;
        for scheme in ["https", "http"] {
            let url = format!("{scheme}://ciscobinary.openh264.org/{file}.bz2");
            match ureq::get(&url).timeout(std::time::Duration::from_secs(60)).call() {
                Ok(r) => {
                    let mut bz = Vec::new();
                    std::io::Read::read_to_end(&mut r.into_reader().take(20 << 20), &mut bz)?;
                    let mut so = Vec::new();
                    std::io::Read::read_to_end(&mut bzip2::read::BzDecoder::new(&bz[..]), &mut so).context("OpenH264: archivio non valido")?;
                    std::fs::create_dir_all("codecs")?;
                    let tmp = lib_path().with_extension("so.part");
                    std::fs::write(&tmp, &so)?;
                    // l'impronta: la controlla `from_blob_path`
                    if let Err(e) = openh264::OpenH264API::from_blob_path(&tmp) {
                        let _ = std::fs::remove_file(&tmp);
                        bail!("OpenH264 scaricato ma non riconosciuto: {e}");
                    }
                    std::fs::rename(&tmp, lib_path())?;
                    return Ok(());
                }
                Err(e) => last = Some(e),
            }
        }
        Err(anyhow!("OpenH264 di Cisco non scaricato: {}", last.map(|e| e.to_string()).unwrap_or_default()))
    }

    use std::io::Read as _;
}

// ---------------------------------------------------------------------------
// OpenH264 → MP4
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "linux", test))]
mod h264 {
    use super::{Frames, FPS};
    use anyhow::{anyhow, Result};
    use openh264::encoder::{BitRate, Encoder, EncoderConfig, FrameRate, IntraFramePeriod, RateControlMode, UsageType};
    use openh264::formats::{RgbaSliceU8, YUVBuffer};
    use std::path::Path;

    pub fn encode(api: openh264::OpenH264API, fr: &Frames, out: &Path) -> Result<()> {
        let cfg = EncoderConfig::new()
            .bitrate(BitRate::from_bps(((fr.w * fr.h * 3) as u32).max(1_000_000)))
            .max_frame_rate(FrameRate::from_hz(FPS as f32))
            // (le modalità «non in tempo reale» OpenH264 non le accetta)
            .usage_type(UsageType::ScreenContentRealTime)
            .rate_control_mode(RateControlMode::Bitrate)
            .intra_frame_period(IntraFramePeriod::from_num_frames(FPS * 2));
        let mut enc = Encoder::with_api_config(api, cfg).map_err(|e| anyhow!("OpenH264: {e}"))?;
        let mut yuv = YUVBuffer::new(fr.w, fr.h);
        let mut mux = super::mp4::Muxer::new(fr.w as u16, fr.h as u16, FPS);
        fr.each(|rgba| {
            yuv.read_rgba8(RgbaSliceU8::new(rgba, (fr.w, fr.h)));
            let bs = enc.encode(&yuv).map_err(|e| anyhow!("OpenH264: {e}"))?;
            let mut nals: Vec<Vec<u8>> = Vec::new();
            for l in 0..bs.num_layers() {
                let Some(layer) = bs.layer(l) else { continue };
                for n in 0..layer.nal_count() {
                    if let Some(nal) = layer.nal_unit(n) {
                        nals.push(super::mp4::strip_start_code(nal).to_vec());
                    }
                }
            }
            mux.frame(nals);
            Ok(())
        })?;
        std::fs::write(out, mux.finish()?)?;
        Ok(())
    }
}

/// Un MP4 minimo con una traccia H.264 (ISO/IEC 14496-12 e -15): quello che
/// serve per i fotogrammi di OpenH264 su Linux.
#[cfg(any(target_os = "linux", test))]
mod mp4 {
    use anyhow::{bail, Result};

    /// Toglie il codice d'inizio di Annex B (00 00 01 o 00 00 00 01).
    pub fn strip_start_code(nal: &[u8]) -> &[u8] {
        if nal.starts_with(&[0, 0, 0, 1]) {
            &nal[4..]
        } else if nal.starts_with(&[0, 0, 1]) {
            &nal[3..]
        } else {
            nal
        }
    }

    pub struct Muxer {
        w: u16,
        h: u16,
        fps: u32,
        sps: Option<Vec<u8>>,
        pps: Option<Vec<u8>>,
        data: Vec<u8>,
        /// (dimensione, durata in fotogrammi, chiave)
        samples: Vec<(u32, u32, bool)>,
    }

    impl Muxer {
        pub fn new(w: u16, h: u16, fps: u32) -> Muxer {
            Muxer { w, h, fps, sps: None, pps: None, data: Vec::new(), samples: Vec::new() }
        }

        /// I NAL di un fotogramma (senza codici d'inizio). Un fotogramma
        /// saltato dal codificatore allunga quello prima.
        pub fn frame(&mut self, nals: Vec<Vec<u8>>) {
            let mut size = 0u32;
            let mut key = false;
            for nal in nals {
                let Some(&head) = nal.first() else { continue };
                match head & 0x1f {
                    7 => {
                        self.sps.get_or_insert(nal);
                    }
                    8 => {
                        self.pps.get_or_insert(nal);
                    }
                    t => {
                        key |= t == 5;
                        self.data.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                        self.data.extend_from_slice(&nal);
                        size += 4 + nal.len() as u32;
                    }
                }
            }
            if size > 0 {
                self.samples.push((size, 1, key));
            } else if let Some(last) = self.samples.last_mut() {
                last.1 += 1;
            }
        }

        pub fn finish(self) -> Result<Vec<u8>> {
            let (Some(sps), Some(pps)) = (&self.sps, &self.pps) else { bail!("H.264 senza SPS/PPS") };
            if sps.len() < 4 || self.samples.is_empty() {
                bail!("H.264 vuoto");
            }
            let timescale = self.fps * 1000;
            let frames: u32 = self.samples.iter().map(|s| s.1).sum();
            let duration = frames * 1000;
            let ftyp = boxed(b"ftyp", &[b"isom".as_slice(), &512u32.to_be_bytes(), b"isom", b"iso2", b"avc1", b"mp41"].concat());
            let moov_with = |offset: u32| -> Vec<u8> {
                // avcC
                let mut avcc = vec![1, sps[1], sps[2], sps[3], 0xff, 0xe1];
                avcc.extend_from_slice(&(sps.len() as u16).to_be_bytes());
                avcc.extend_from_slice(sps);
                avcc.push(1);
                avcc.extend_from_slice(&(pps.len() as u16).to_be_bytes());
                avcc.extend_from_slice(pps);
                let mut avc1 = vec![0u8; 6];
                avc1.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
                avc1.extend_from_slice(&[0u8; 16]);
                avc1.extend_from_slice(&self.w.to_be_bytes());
                avc1.extend_from_slice(&self.h.to_be_bytes());
                avc1.extend_from_slice(&0x0048_0000u32.to_be_bytes());
                avc1.extend_from_slice(&0x0048_0000u32.to_be_bytes());
                avc1.extend_from_slice(&0u32.to_be_bytes());
                avc1.extend_from_slice(&1u16.to_be_bytes()); // frame_count
                avc1.extend_from_slice(&[0u8; 32]); // compressorname
                avc1.extend_from_slice(&0x0018u16.to_be_bytes());
                avc1.extend_from_slice(&(-1i16).to_be_bytes());
                avc1.extend_from_slice(&boxed(b"avcC", &avcc));
                let stsd = full(b"stsd", 0, &[&1u32.to_be_bytes()[..], &boxed(b"avc1", &avc1)].concat());
                // stts: durate (a gruppi uguali)
                let mut runs: Vec<(u32, u32)> = Vec::new();
                for s in &self.samples {
                    let d = s.1 * 1000;
                    match runs.last_mut() {
                        Some(r) if r.1 == d => r.0 += 1,
                        _ => runs.push((1, d)),
                    }
                }
                let mut stts = (runs.len() as u32).to_be_bytes().to_vec();
                for (n, d) in runs {
                    stts.extend_from_slice(&n.to_be_bytes());
                    stts.extend_from_slice(&d.to_be_bytes());
                }
                let keys: Vec<u32> = self.samples.iter().enumerate().filter(|(_, s)| s.2).map(|(i, _)| i as u32 + 1).collect();
                let mut stss = (keys.len() as u32).to_be_bytes().to_vec();
                for k in keys {
                    stss.extend_from_slice(&k.to_be_bytes());
                }
                let n = self.samples.len() as u32;
                let stsc = [1u32, 1, n, 1].iter().flat_map(|v| v.to_be_bytes()).collect::<Vec<u8>>();
                let mut stsz = [0u32, n].iter().flat_map(|v| v.to_be_bytes()).collect::<Vec<u8>>();
                for s in &self.samples {
                    stsz.extend_from_slice(&s.0.to_be_bytes());
                }
                let stco = [1u32, offset].iter().flat_map(|v| v.to_be_bytes()).collect::<Vec<u8>>();
                let stbl = boxed(
                    b"stbl",
                    &[stsd, full(b"stts", 0, &stts), full(b"stss", 0, &stss), full(b"stsc", 0, &stsc), full(b"stsz", 0, &stsz), full(b"stco", 0, &stco)].concat(),
                );
                let dref = full(b"dref", 0, &[&1u32.to_be_bytes()[..], &full(b"url ", 1, &[])].concat());
                let minf = boxed(b"minf", &[full(b"vmhd", 1, &[0u8; 8]), boxed(b"dinf", &dref), stbl].concat());
                let mut hdlr = vec![0u8; 4];
                hdlr.extend_from_slice(b"vide");
                hdlr.extend_from_slice(&[0u8; 12]);
                hdlr.extend_from_slice(b"OpenSlate\0");
                let mut mdhd = Vec::new();
                for v in [0u32, 0, timescale, duration] {
                    mdhd.extend_from_slice(&v.to_be_bytes());
                }
                mdhd.extend_from_slice(&0x55c4u16.to_be_bytes()); // lingua: «und»
                mdhd.extend_from_slice(&0u16.to_be_bytes());
                let mdia = boxed(b"mdia", &[full(b"mdhd", 0, &mdhd), full(b"hdlr", 0, &hdlr), minf].concat());
                let matrix: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];
                let mut tkhd = Vec::new();
                for v in [0u32, 0, 1, 0, duration, 0, 0] {
                    tkhd.extend_from_slice(&v.to_be_bytes());
                }
                tkhd.extend_from_slice(&[0u8; 4]); // layer, alternate_group
                tkhd.extend_from_slice(&[0u8; 4]); // volume, reserved
                for m in matrix {
                    tkhd.extend_from_slice(&m.to_be_bytes());
                }
                tkhd.extend_from_slice(&((self.w as u32) << 16).to_be_bytes());
                tkhd.extend_from_slice(&((self.h as u32) << 16).to_be_bytes());
                let trak = boxed(b"trak", &[full(b"tkhd", 3, &tkhd), mdia].concat());
                let mut mvhd = Vec::new();
                for v in [0u32, 0, timescale, duration, 0x0001_0000] {
                    mvhd.extend_from_slice(&v.to_be_bytes());
                }
                mvhd.extend_from_slice(&0x0100u16.to_be_bytes());
                mvhd.extend_from_slice(&[0u8; 10]);
                for m in matrix {
                    mvhd.extend_from_slice(&m.to_be_bytes());
                }
                mvhd.extend_from_slice(&[0u8; 24]);
                mvhd.extend_from_slice(&2u32.to_be_bytes()); // next_track_ID
                boxed(b"moov", &[full(b"mvhd", 0, &mvhd), trak].concat())
            };
            // il moov prima dei dati (si riproduce subito, anche dal web)
            let moov_len = moov_with(0).len() as u32;
            let offset = ftyp.len() as u32 + moov_len + 8;
            let mut out = ftyp;
            out.extend_from_slice(&moov_with(offset));
            out.extend_from_slice(&(self.data.len() as u32 + 8).to_be_bytes());
            out.extend_from_slice(b"mdat");
            out.extend_from_slice(&self.data);
            Ok(out)
        }
    }

    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }

    fn full(kind: &[u8; 4], flags: u32, body: &[u8]) -> Vec<u8> {
        boxed(kind, &[&flags.to_be_bytes()[..], body].concat())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::PageStroke;

    fn sample() -> Page {
        let strokes = (0..12)
            .map(|k| {
                let v = 0.1 + k as f32 * 0.06;
                PageStroke { color: [20, 20, 40], width: 2.0, pts: vec![(0.1, v), (0.5, v + 0.02), (0.8, v)], highlight: false }
            })
            .collect();
        Page { w: 21000.0, h: 29700.0, strokes, bg: None }
    }

    /// Il video con OpenH264 (compilato dai sorgenti, solo per le prove) e il
    /// nostro MP4: ffprobe (se c'è) lo deve leggere tutto.
    #[test]
    fn mp4_con_openh264() {
        let page = sample();
        let fr = Frames::new(&page).unwrap();
        assert_eq!((fr.w % 2, fr.h % 2), (0, 0));
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("v.mp4");
        h264::encode(openh264::OpenH264API::from_source(), &fr, &out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[4..8], b"ftyp");
        if let Some(keep) = std::env::var_os("VIDEO_OUT") {
            std::fs::copy(&out, keep).unwrap();
        }
        assert!(bytes.windows(4).any(|w| w == b"avcC"));
        if let Ok(o) = std::process::Command::new("ffprobe")
            .args(["-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", "stream=codec_name,width,height,nb_read_frames", "-of", "csv=p=0"])
            .arg(&out)
            .output()
        {
            let s = String::from_utf8_lossy(&o.stdout);
            assert!(o.status.success() && s.starts_with("h264,"), "{s} {}", String::from_utf8_lossy(&o.stderr));
            let n: usize = s.trim().rsplit(',').next().unwrap().parse().unwrap();
            assert!(n >= Frames::count() / 2, "{s}");
        }
    }

    /// Su Linux, con ffmpeg (se c'è con l'H.264).
    #[cfg(target_os = "linux")]
    #[test]
    fn mp4_con_ffmpeg() {
        let Some(codec) = ffmpeg_h264() else { return };
        let page = sample();
        let fr = Frames::new(&page).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("f.mp4");
        ffmpeg(&fr, &out, codec).unwrap();
        assert!(std::fs::metadata(&out).unwrap().len() > 1000);
    }
}
