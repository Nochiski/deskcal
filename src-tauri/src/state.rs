//! Process-wide state shared by commands, tray, and background tasks.
use crate::google::AccessToken;
use crate::model::{Accounts, Settings, SyncResult, WindowGeometry};
use crate::store::{self, Paths};
use chrono::{Duration, Local, NaiveDate};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Mutex;
use tauri::AppHandle;

pub struct AppState {
    pub paths: Paths,
    pub http: reqwest::Client,
    pub settings: Mutex<Settings>,
    pub accounts: Mutex<Accounts>,
    pub cache: Mutex<SyncResult>,
    pub google_token: Mutex<Option<AccessToken>>,
    /// Last date range requested by the UI (used by the background sync).
    pub last_range: Mutex<(NaiveDate, NaiveDate)>,
    /// Reminder keys that have already fired ("eventId|minutes").
    pub fired: Mutex<HashSet<String>>,
    pub syncing: AtomicBool,
    pub pending_geometry: Mutex<WindowGeometry>,
    pub geometry_seq: AtomicU64,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Self {
        let paths = Paths::new(app);
        let settings = store::load_settings(&paths);
        let accounts = store::load_accounts(&paths);
        let cache = store::load_cache(&paths);
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
            google_token: Mutex::new(None),
            last_range: Mutex::new((today - Duration::days(45), today + Duration::days(60))),
            fired: Mutex::new(HashSet::new()),
            syncing: AtomicBool::new(false),
            pending_geometry: Mutex::new(WindowGeometry::default()),
            geometry_seq: AtomicU64::new(0),
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
