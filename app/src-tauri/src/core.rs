//! Safe wrapper over the C API of TorminCore (core/tormin_core.h), loaded at runtime.

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

use libloading::Library;
use serde::Serialize;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Status {
    pub state: i32,
    pub paused: i32,
    pub has_metadata: i32,
    pub progress: f32,
    pub download_rate: i64,
    pub upload_rate: i64,
    pub total_wanted: i64,
    pub total_wanted_done: i64,
    pub num_peers: i32,
    pub num_seeds: i32,
}

#[repr(C)]
struct RawEvent {
    kind: i32,
    torrent: u32,
    message: *const c_char,
    data: *const u8,
    data_len: i64,
}

pub enum Event {
    MetadataReceived(u32),
    ResumeData(u32, Vec<u8>),
    ResumeFailed(String),
    Error(String),
    Finished(u32),
    TrackerError(String),
    Log(String),
}

pub const STATE_NAMES: [&str; 6] = ["unknown", "checking", "metadata", "downloading", "finished", "seeding"];

#[derive(Serialize, Clone)]
pub struct FileEntry {
    pub index: usize,
    pub path: String,
    pub size: i64,
}

#[derive(Serialize, Clone)]
pub struct Metadata {
    pub name: String,
    pub total_size: i64,
    pub files: Vec<FileEntry>,
}

type Ptr = *mut c_void;

/// Function table of TorminCore. The library stays loaded for the whole process.
struct Api {
    _lib: Library,
    version: unsafe extern "C" fn() -> *const c_char,
    last_error: unsafe extern "C" fn() -> *const c_char,
    session_create: unsafe extern "C" fn(*const c_char) -> Ptr,
    session_destroy: unsafe extern "C" fn(Ptr),
    next_event: unsafe extern "C" fn(Ptr, *mut RawEvent) -> c_int,
    wait_event: unsafe extern "C" fn(Ptr, c_int) -> c_int,
    add_magnet: unsafe extern "C" fn(Ptr, *const c_char, *const c_char, c_int) -> u32,
    add_torrent_file: unsafe extern "C" fn(Ptr, *const c_char, *const c_char, c_int, *const u8, c_int) -> u32,
    add_resume_data: unsafe extern "C" fn(Ptr, *const u8, i64, *const c_char) -> u32,
    torrent_status: unsafe extern "C" fn(Ptr, u32, *mut Status) -> c_int,
    torrent_name: unsafe extern "C" fn(Ptr, u32) -> *const c_char,
    torrent_key: unsafe extern "C" fn(Ptr, u32) -> *const c_char,
    torrent_save_path: unsafe extern "C" fn(Ptr, u32) -> *const c_char,
    torrent_pause: unsafe extern "C" fn(Ptr, u32) -> c_int,
    torrent_resume: unsafe extern "C" fn(Ptr, u32) -> c_int,
    torrent_remove: unsafe extern "C" fn(Ptr, u32, c_int) -> c_int,
    torrent_save_resume: unsafe extern "C" fn(Ptr, u32) -> c_int,
    torrent_need_save_resume: unsafe extern "C" fn(Ptr, u32) -> c_int,
    torrent_set_download_limit: unsafe extern "C" fn(Ptr, u32, c_int) -> c_int,
    torrent_download_limit: unsafe extern "C" fn(Ptr, u32) -> c_int,
    torrent_get_priorities: unsafe extern "C" fn(Ptr, u32, *mut u8, c_int) -> c_int,
    torrent_set_priorities: unsafe extern "C" fn(Ptr, u32, *const u8, c_int) -> c_int,
    torrent_info: unsafe extern "C" fn(Ptr, u32) -> Ptr,
    tinfo_load: unsafe extern "C" fn(*const c_char) -> Ptr,
    tinfo_free: unsafe extern "C" fn(Ptr),
    tinfo_name: unsafe extern "C" fn(Ptr) -> *const c_char,
    tinfo_num_files: unsafe extern "C" fn(Ptr) -> c_int,
    tinfo_file_path: unsafe extern "C" fn(Ptr, c_int) -> *const c_char,
    tinfo_file_size: unsafe extern "C" fn(Ptr, c_int) -> i64,
    tinfo_total_size: unsafe extern "C" fn(Ptr) -> i64,
}

fn text(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// A libtorrent session. All calls come from behind the engine mutex.
pub struct Session {
    api: Api,
    ptr: Ptr,
}

// TorminCore guards the session with its own mutex; the pointer itself is just an opaque handle.
unsafe impl Send for Session {}

impl Session {
    pub fn open(library: &Path) -> Result<Self, String> {
        let api = unsafe { load_api(library) }.map_err(|e| format!("{}: {e}", library.display()))?;
        let ptr = unsafe { (api.session_create)(std::ptr::null()) };
        if ptr.is_null() {
            return Err(format!("Could not start libtorrent: {}", text(unsafe { (api.last_error)() })));
        }
        Ok(Session { api, ptr })
    }

    pub fn version(&self) -> String {
        text(unsafe { (self.api.version)() })
    }

    fn last_error(&self) -> String {
        text(unsafe { (self.api.last_error)() })
    }

    fn id_or_error(&self, id: u32) -> Result<u32, String> {
        if id == 0 { Err(self.last_error()) } else { Ok(id) }
    }

    pub fn add_magnet(&self, uri: &str, save_path: &str, paused: bool) -> Result<u32, String> {
        let id = unsafe { (self.api.add_magnet)(self.ptr, cstr(uri).as_ptr(), cstr(save_path).as_ptr(), paused as c_int) };
        self.id_or_error(id)
    }

    pub fn add_torrent_file(&self, path: &str, save_path: &str, paused: bool, priorities: Option<&[u8]>) -> Result<u32, String> {
        let (prio, count) = priorities.map_or((std::ptr::null(), 0), |p| (p.as_ptr(), p.len() as c_int));
        let id = unsafe {
            (self.api.add_torrent_file)(self.ptr, cstr(path).as_ptr(), cstr(save_path).as_ptr(), paused as c_int, prio, count)
        };
        self.id_or_error(id)
    }

    pub fn add_resume_data(&self, data: &[u8], save_path: &str) -> Result<u32, String> {
        let id = unsafe { (self.api.add_resume_data)(self.ptr, data.as_ptr(), data.len() as i64, cstr(save_path).as_ptr()) };
        self.id_or_error(id)
    }

    pub fn status(&self, id: u32) -> Option<Status> {
        let mut st = Status::default();
        (unsafe { (self.api.torrent_status)(self.ptr, id, &mut st) } == 0).then_some(st)
    }

    pub fn name(&self, id: u32) -> String {
        text(unsafe { (self.api.torrent_name)(self.ptr, id) })
    }

    pub fn key(&self, id: u32) -> String {
        text(unsafe { (self.api.torrent_key)(self.ptr, id) })
    }

    pub fn save_path(&self, id: u32) -> String {
        text(unsafe { (self.api.torrent_save_path)(self.ptr, id) })
    }

    pub fn pause(&self, id: u32) {
        unsafe { (self.api.torrent_pause)(self.ptr, id) };
    }

    pub fn resume(&self, id: u32) {
        unsafe { (self.api.torrent_resume)(self.ptr, id) };
    }

    pub fn remove(&self, id: u32, delete_files: bool) {
        unsafe { (self.api.torrent_remove)(self.ptr, id, delete_files as c_int) };
    }

    pub fn request_resume_data(&self, id: u32) -> bool {
        unsafe { (self.api.torrent_save_resume)(self.ptr, id) == 0 }
    }

    pub fn need_save_resume(&self, id: u32) -> bool {
        unsafe { (self.api.torrent_need_save_resume)(self.ptr, id) == 1 }
    }

    pub fn set_download_limit(&self, id: u32, bytes_per_sec: i32) {
        unsafe { (self.api.torrent_set_download_limit)(self.ptr, id, bytes_per_sec) };
    }

    pub fn download_limit(&self, id: u32) -> i32 {
        unsafe { (self.api.torrent_download_limit)(self.ptr, id) }.max(0)
    }

    pub fn priorities(&self, id: u32, count: usize) -> Vec<u8> {
        let mut buf = vec![4u8; count];
        let n = unsafe { (self.api.torrent_get_priorities)(self.ptr, id, buf.as_mut_ptr(), count as c_int) };
        if n < 0 {
            return vec![4; count];
        }
        buf
    }

    pub fn set_priorities(&self, id: u32, priorities: &[u8]) {
        unsafe { (self.api.torrent_set_priorities)(self.ptr, id, priorities.as_ptr(), priorities.len() as c_int) };
    }

    pub fn metadata(&self, id: u32) -> Option<Metadata> {
        let ti = unsafe { (self.api.torrent_info)(self.ptr, id) };
        self.read_tinfo(ti)
    }

    pub fn load_torrent_file(&self, path: &str) -> Result<Metadata, String> {
        let ti = unsafe { (self.api.tinfo_load)(cstr(path).as_ptr()) };
        self.read_tinfo(ti).ok_or_else(|| self.last_error())
    }

    fn read_tinfo(&self, ti: Ptr) -> Option<Metadata> {
        if ti.is_null() {
            return None;
        }
        let a = &self.api;
        let meta = unsafe {
            let n = (a.tinfo_num_files)(ti).max(0);
            Metadata {
                name: text((a.tinfo_name)(ti)),
                total_size: (a.tinfo_total_size)(ti),
                files: (0..n)
                    .map(|i| FileEntry {
                        index: i as usize,
                        path: text((a.tinfo_file_path)(ti, i)),
                        size: (a.tinfo_file_size)(ti, i),
                    })
                    .collect(),
            }
        };
        unsafe { (a.tinfo_free)(ti) };
        Some(meta)
    }

    pub fn next_event(&self) -> Option<Event> {
        let mut ev = RawEvent { kind: 0, torrent: 0, message: std::ptr::null(), data: std::ptr::null(), data_len: 0 };
        if unsafe { (self.api.next_event)(self.ptr, &mut ev) } != 1 {
            return None;
        }
        let msg = text(ev.message);
        let t = ev.torrent;
        Some(match ev.kind {
            1 => Event::MetadataReceived(t),
            2 => {
                let data = if ev.data.is_null() || ev.data_len <= 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(ev.data, ev.data_len as usize) }.to_vec()
                };
                Event::ResumeData(t, data)
            }
            3 => Event::ResumeFailed(msg),
            4 => Event::Error(msg),
            5 => Event::Finished(t),
            6 => Event::TrackerError(msg),
            _ => Event::Log(msg),
        })
    }

    pub fn wait_event(&self, timeout_ms: i32) -> bool {
        unsafe { (self.api.wait_event)(self.ptr, timeout_ms) == 1 }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { (self.api.session_destroy)(self.ptr) };
    }
}

unsafe fn load_api(path: &Path) -> Result<Api, libloading::Error> {
    unsafe {
        let lib = Library::new(path)?;
        macro_rules! f {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())?
            };
        }
        Ok(Api {
            version: f!("tm_version"),
            last_error: f!("tm_last_error"),
            session_create: f!("tm_session_create"),
            session_destroy: f!("tm_session_destroy"),
            next_event: f!("tm_next_event"),
            wait_event: f!("tm_wait_event"),
            add_magnet: f!("tm_add_magnet"),
            add_torrent_file: f!("tm_add_torrent_file"),
            add_resume_data: f!("tm_add_resume_data"),
            torrent_status: f!("tm_torrent_status"),
            torrent_name: f!("tm_torrent_name"),
            torrent_key: f!("tm_torrent_key"),
            torrent_save_path: f!("tm_torrent_save_path"),
            torrent_pause: f!("tm_torrent_pause"),
            torrent_resume: f!("tm_torrent_resume"),
            torrent_remove: f!("tm_torrent_remove"),
            torrent_save_resume: f!("tm_torrent_save_resume"),
            torrent_need_save_resume: f!("tm_torrent_need_save_resume"),
            torrent_set_download_limit: f!("tm_torrent_set_download_limit"),
            torrent_download_limit: f!("tm_torrent_download_limit"),
            torrent_get_priorities: f!("tm_torrent_get_priorities"),
            torrent_set_priorities: f!("tm_torrent_set_priorities"),
            torrent_info: f!("tm_torrent_info"),
            tinfo_load: f!("tm_tinfo_load"),
            tinfo_free: f!("tm_tinfo_free"),
            tinfo_name: f!("tm_tinfo_name"),
            tinfo_num_files: f!("tm_tinfo_num_files"),
            tinfo_file_path: f!("tm_tinfo_file_path"),
            tinfo_file_size: f!("tm_tinfo_file_size"),
            tinfo_total_size: f!("tm_tinfo_total_size"),
            _lib: lib,
        })
    }
}
