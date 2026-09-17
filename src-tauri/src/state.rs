//! Process-wide state shared by commands, tray, and background tasks.
use crate::google::AccessToken;
use crate::model::{Accounts, Settings, SyncResult, WindowGeometry};
use crate::store::{self, Paths};
use chrono::{Duration, Local, NaiveDate};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;
use tauri::AppHandle;

pub struct AppState {
    pub paths: Paths,
    pub http: reqwest::Client,
    pub settings: Mutex<Settings>,
    pub accounts: Mutex<Accounts>,
    pub cache: Mutex<SyncResult>,
    /// Access tokens per Google account id.
    pub google_token: Mutex<HashMap<String, AccessToken>>,
    /// Last date range requested by the UI (used by the background sync).
    pub last_range: Mutex<(NaiveDate, NaiveDate)>,
    /// Reminder keys that have already fired ("eventId|minutes").
    pub fired: Mutex<HashSet<String>>,
    /// Serialize sync and invitation responses so an older fetch cannot overwrite a new RSVP.
    pub sync_lock: tokio::sync::Mutex<()>,
    pub pending_geometry: Mutex<WindowGeometry>,
    pub geometry_seq: AtomicU64,
    /// Google login in progress: cancel flag + loopback port (to wake the listener).
    pub login_cancel: std::sync::Arc<AtomicBool>,
    pub login_port: Mutex<Option<u16>>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Self {
        let paths = Paths::new(app);
        let mut settings = store::load_settings(&paths);
        let mut accounts = store::load_accounts(&paths);
        let mut cache = store::load_cache(&paths);
        store::migrate_accounts(&paths, &mut accounts, &mut settings, &mut cache);
        let today = Local::now().date_naive();
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(40))
            .build()
            .expect("http client");
        Self {
            paths,
            http,
            settings: Mutex::new(settings),
            accounts: Mutex::new(accounts),
            cache: Mutex::new(cache),
            google_token: Mutex::new(HashMap::new()),
            last_range: Mutex::new((today - Duration::days(45), today + Duration::days(60))),
            fired: Mutex::new(HashSet::new()),
            sync_lock: tokio::sync::Mutex::new(()),
            pending_geometry: Mutex::new(WindowGeometry::default()),
            geometry_seq: AtomicU64::new(0),
            login_cancel: std::sync::Arc::new(AtomicBool::new(false)),
            login_port: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
    pub fn accounts(&self) -> Accounts {
        self.accounts.lock().unwrap().clone()
    }
    pub fn set_settings(&self, s: Settings) {
        store::save_settings(&self.paths, &s);
        *self.settings.lock().unwrap() = s;
    }
    pub fn set_accounts(&self, a: Accounts) {
        store::save_accounts(&self.paths, &a);
        *self.accounts.lock().unwrap() = a;
    }
    pub fn set_cache(&self, c: SyncResult) {
        store::save_cache(&self.paths, &c);
        *self.cache.lock().unwrap() = c;
    }
}
