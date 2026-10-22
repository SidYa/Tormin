//! Torrent list on top of the libtorrent session: adding, persistence, polling.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::config::{self, Config, SessionTorrent};
use crate::core::{Event, Metadata, STATE_NAMES, Session};

struct Item {
    id: u32,
    key: String,
    source: Option<String>,
    is_magnet: bool,
    file_priorities: Option<Vec<u8>>,
    /// Magnet added by the user: ask which files to download once metadata arrives
    select_files_on_metadata: bool,
    finished: bool,
}

/// One row of the torrent list as the UI sees it.
#[derive(Serialize, Clone)]
pub struct TorrentView {
    pub id: u32,
    pub name: String,
    pub state: &'static str,
    pub progress: f32,
    pub paused: bool,
    pub finished: bool,
    pub has_metadata: bool,
    pub download_rate: i64,
    pub upload_rate: i64,
    pub total: i64,
    pub done: i64,
    pub peers: i32,
    pub seeds: i32,
}

#[derive(Serialize, Clone)]
pub struct Snapshot {
    pub torrents: Vec<TorrentView>,
    pub download_rate: i64,
    pub upload_rate: i64,
}

/// Things that happened during a tick and need the UI or the OS.
#[derive(Default)]
pub struct TickOutcome {
    pub select_files: Vec<(u32, Metadata)>,
    pub finished: Vec<String>,
    pub errors: Vec<String>,
    pub all_completed: bool,
}

#[derive(Serialize, Clone)]
pub struct Details {
    pub id: u32,
    pub name: String,
    pub save_path: String,
    pub download_limit_kb: i32,
    pub metadata: Option<Metadata>,
    pub priorities: Vec<u8>,
}

pub struct Engine {
    session: Session,
    pub config: Config,
    items: Vec<Item>,
    ticks: u64,
}

fn resume_path(key: &str) -> PathBuf {
    config::resume_dir().join(format!("{key}.fastresume"))
}

fn normalize_magnet(uri: &str) -> String {
    // Some browsers send magnet:///?xt= or magnet://?xt= instead of magnet:?xt=
    let uri = uri.trim();
    for prefix in ["magnet:///", "magnet://"] {
        if uri.len() >= prefix.len() && uri[..prefix.len()].eq_ignore_ascii_case(prefix) {
            return format!("magnet:{}", &uri[prefix.len()..]);
        }
    }
    uri.to_string()
}

impl Engine {
    pub fn start(library: &Path, mut config: Config) -> Result<Engine, String> {
        let _ = fs::create_dir_all(config::resume_dir());
        let session = Session::open(library)?;
        crate::log(&format!("libtorrent {} session started", session.version()));
        // Never shut the PC down because the option was left on (same as the other versions)
        if config.shutdown_complete {
            config.shutdown_complete = false;
            config.save();
        }
        let mut engine = Engine { session, config, items: Vec::new(), ticks: 0 };
        engine.load_session();
        Ok(engine)
    }

    pub fn version(&self) -> String {
        self.session.version()
    }

    // ------------------------------------------------------------- adding

    fn track(&mut self, id: u32, source: Option<String>, is_magnet: bool) -> &mut Item {
        // Adding a torrent that is already in the session returns its existing id
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            return &mut self.items[pos];
        }
        let finished = self.session.status(id).is_some_and(|s| s.state >= 4 && s.progress >= 1.0);
        self.items.push(Item {
            id,
            key: self.session.key(id),
            source,
            is_magnet,
            file_priorities: None,
            select_files_on_metadata: false,
            finished,
        });
        self.items.last_mut().unwrap()
    }

    pub fn add_magnet(&mut self, uri: &str, interactive: bool, paused: bool) -> Result<u32, String> {
        let uri = normalize_magnet(uri);
        let dir = self.config.download_dir();
        let id = self.session.add_magnet(&uri, &dir, paused)?;
        self.track(id, Some(uri), true).select_files_on_metadata = interactive;
        self.save_session(true);
        Ok(id)
    }

    pub fn add_torrent_file(&mut self, path: &str, priorities: Option<Vec<u8>>, paused: bool) -> Result<u32, String> {
        let dir = self.config.download_dir();
        let id = self.session.add_torrent_file(path, &dir, paused, priorities.as_deref())?;
        let full = fs::canonicalize(path)
            .map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
            .unwrap_or_else(|_| path.to_string());
        self.track(id, Some(full), false).file_priorities = priorities;
        self.save_session(true);
        Ok(id)
    }

    pub fn read_torrent_file(&self, path: &str) -> Result<Metadata, String> {
        self.session.load_torrent_file(path)
    }

    // --------------------------------------------------------- operations

    pub fn pause(&self, ids: &[u32]) {
        ids.iter().for_each(|&id| self.session.pause(id));
    }

    pub fn resume(&self, ids: &[u32]) {
        ids.iter().for_each(|&id| self.session.resume(id));
    }

    pub fn all_ids(&self) -> Vec<u32> {
        self.items.iter().map(|i| i.id).collect()
    }

    pub fn remove(&mut self, ids: &[u32], delete_files: bool) {
        for &id in ids {
            self.session.remove(id, delete_files);
            if let Some(pos) = self.items.iter().position(|i| i.id == id) {
                let item = self.items.remove(pos);
                let _ = fs::remove_file(resume_path(&item.key));
                // "Yes" in the delete dialog removes everything, the .torrent file included
                if delete_files && !item.is_magnet {
                    if let Some(source) = &item.source {
                        let _ = fs::remove_file(source);
                    }
                }
            }
        }
        self.save_session(true);
        self.config.save();
    }

    pub fn save_path(&self, id: u32) -> String {
        self.session.save_path(id)
    }

    pub fn name(&self, id: u32) -> String {
        self.session.name(id)
    }

    pub fn details(&self, id: u32) -> Details {
        let metadata = self.session.metadata(id);
        let count = metadata.as_ref().map_or(0, |m| m.files.len());
        let stored = self.items.iter().find(|i| i.id == id).and_then(|i| i.file_priorities.clone());
        let priorities = match stored {
            Some(p) if p.len() == count => p,
            _ => self.session.priorities(id, count),
        };
        Details {
            id,
            name: self.session.name(id),
            save_path: self.session.save_path(id),
            download_limit_kb: self.session.download_limit(id) / 1024,
            metadata,
            priorities,
        }
    }

    pub fn apply_properties(&mut self, id: u32, download_limit_kb: i32, priorities: Option<Vec<u8>>) {
        self.session.set_download_limit(id, download_limit_kb.max(0).saturating_mul(1024));
        if let Some(p) = priorities {
            self.set_priorities(id, p);
        }
    }

    fn set_priorities(&mut self, id: u32, priorities: Vec<u8>) {
        self.session.set_priorities(id, &priorities);
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.file_priorities = Some(priorities);
        }
    }

    /// The user picked files for a magnet whose metadata just arrived (None = cancelled).
    pub fn complete_file_selection(&mut self, id: u32, priorities: Option<Vec<u8>>) {
        match priorities {
            Some(p) => {
                self.set_priorities(id, p);
                self.session.resume(id);
            }
            // Cancelling the file selection of a magnet means not adding it at all
            None => self.remove(&[id], false),
        }
    }

    // ------------------------------------------------------------ polling

    pub fn snapshot(&self) -> Snapshot {
        let mut torrents = Vec::with_capacity(self.items.len());
        let (mut down, mut up) = (0, 0);
        for item in &self.items {
            let Some(st) = self.session.status(item.id) else { continue };
            down += st.download_rate;
            up += st.upload_rate;
            let name = self.session.name(item.id);
            torrents.push(TorrentView {
                id: item.id,
                name: if name.is_empty() { "Retrieving Metadata...".into() } else { name },
                state: STATE_NAMES.get(st.state as usize).copied().unwrap_or("unknown"),
                progress: st.progress,
                paused: st.paused != 0,
                finished: st.progress >= 1.0 && (st.state == 4 || st.state == 5),
                has_metadata: st.has_metadata != 0,
                download_rate: st.download_rate,
                upload_rate: st.upload_rate,
                total: st.total_wanted,
                done: st.total_wanted_done,
                peers: st.num_peers,
                seeds: st.num_seeds,
            });
        }
        Snapshot { torrents, download_rate: down, upload_rate: up }
    }

    pub fn tick(&mut self) -> TickOutcome {
        let mut out = TickOutcome::default();
        self.process_events(Some(&mut out));

        // Periodically persist the session so a crash does not lose progress
        self.ticks += 1;
        if self.ticks % 60 == 0 {
            self.save_session(true);
            self.config.save();
        }

        if self.config.shutdown_complete && !self.items.is_empty() {
            let snap = self.snapshot();
            if snap.torrents.iter().all(|t| t.finished) {
                // Fire only once
                self.config.shutdown_complete = false;
                self.config.save();
                out.all_completed = true;
            }
        }
        out
    }

    /// Returns the number of finished resume data requests.
    fn process_events(&mut self, mut out: Option<&mut TickOutcome>) -> usize {
        let mut finished_requests = 0;
        while let Some(event) = self.session.next_event() {
            match event {
                Event::ResumeData(id, data) => {
                    finished_requests += 1;
                    if let Some(item) = self.items.iter().find(|i| i.id == id) {
                        write_resume_data(&item.key, &data);
                    }
                }
                Event::ResumeFailed(msg) => {
                    finished_requests += 1;
                    crate::log(&format!("Resume data failed: {msg}"));
                }
                Event::MetadataReceived(id) => {
                    if let Some(out) = out.as_deref_mut() {
                        self.on_metadata_received(id, out);
                    }
                }
                Event::Finished(id) => {
                    if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                        // Restored torrents that were already complete do not notify again
                        if !item.finished {
                            item.finished = true;
                            if let Some(out) = out.as_deref_mut() {
                                out.finished.push(self.session.name(id));
                            }
                        }
                    }
                }
                Event::Error(msg) => {
                    crate::log(&format!("Torrent error: {msg}"));
                    if let Some(out) = out.as_deref_mut() {
                        out.errors.push(msg);
                    }
                }
                Event::TrackerError(msg) => {
                    // Announces from loopback/virtual adapters always fail; that is just noise
                    if !msg.to_ascii_lowercase().contains("unreachable") {
                        crate::log(&format!("Tracker: {msg}"));
                    }
                }
                Event::Log(msg) => crate::log(&format!("lt: {msg}")),
            }
        }
        finished_requests
    }

    fn on_metadata_received(&mut self, id: u32, out: &mut TickOutcome) {
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else { return };
        if !item.select_files_on_metadata {
            return;
        }
        item.select_files_on_metadata = false;
        if let Some(meta) = self.session.metadata(id) {
            if meta.files.len() > 1 {
                // Hold the download until the user picks the files
                self.session.pause(id);
                out.select_files.push((id, meta));
            }
        }
    }

    // -------------------------------------------------------- persistence

    /// Store the torrent list in config and request resume data (written when the events arrive).
    pub fn save_session(&mut self, only_if_needed: bool) -> usize {
        let mut list = Vec::with_capacity(self.items.len());
        let mut requested = 0;
        for item in &self.items {
            let paused = self.session.status(item.id).is_some_and(|s| s.paused != 0);
            list.push(SessionTorrent {
                source: item.source.clone(),
                is_magnet: item.is_magnet,
                paused,
                key: Some(item.key.clone()),
                file_priorities: item.file_priorities.clone(),
            });
            if (!only_if_needed || self.session.need_save_resume(item.id)) && self.session.request_resume_data(item.id) {
                requested += 1;
            }
        }
        self.config.torrents = list;
        requested
    }

    /// Save everything and block until the resume data is on disk (used on quit).
    pub fn save_session_sync(&mut self, timeout: Duration) {
        let mut pending = self.save_session(false);
        let start = Instant::now();
        while pending > 0 {
            let left = timeout.saturating_sub(start.elapsed());
            if left.is_zero() {
                crate::log(&format!("Timed out waiting for resume data ({pending} left)"));
                break;
            }
            if self.session.wait_event(left.as_millis() as i32) {
                pending = pending.saturating_sub(self.process_events(None));
            }
        }
        self.config.save();
    }

    fn load_session(&mut self) {
        let entries = self.config.torrents.clone();
        let dir = self.config.download_dir();
        for entry in entries {
            if let Some(key) = &entry.key {
                if let Ok(data) = fs::read(resume_path(key)) {
                    match self.session.add_resume_data(&data, &dir) {
                        Ok(id) => {
                            self.track(id, entry.source.clone(), entry.is_magnet).file_priorities = entry.file_priorities.clone();
                            continue;
                        }
                        Err(e) => crate::log(&format!("Error loading resume data for {key}: {e}")),
                    }
                }
            }
            let Some(source) = entry.source.clone() else { continue };
            let result = if entry.is_magnet {
                self.add_magnet(&source, false, entry.paused)
            } else if Path::new(&source).exists() {
                self.add_torrent_file(&source, entry.file_priorities.clone(), entry.paused)
            } else {
                Err(format!("File not found: {source}"))
            };
            if let Err(e) = result {
                crate::log(&format!("Error loading torrent from session: {e}"));
            }
        }
    }
}

fn write_resume_data(key: &str, data: &[u8]) {
    let path = resume_path(key);
    let tmp = path.with_extension("tmp");
    if let Err(e) = fs::write(&tmp, data).and_then(|_| fs::rename(&tmp, &path)) {
        crate::log(&format!("Error writing resume data: {e}"));
    }
}
