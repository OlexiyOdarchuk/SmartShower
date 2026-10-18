//! Стан, спільний для циклу заліза і бот-потоку.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard};

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use log::{info, warn};
use shower_core::queue::{JoinResult, Member, Queue};
use shower_core::shower::WaterTemperature;
use shower_core::texts;

const NVS_NAMESPACE: &str = "ss";
const NVS_QUEUE_KEY: &str = "queue";

const BEEP_NONE: u8 = 0;
const BEEP_OK: u8 = 1;
const BEEP_ERROR: u8 = 2;

struct Inner {
    queue: Queue,
    water: [Option<WaterTemperature>; 2],
    nvs: EspNvs<NvsDefault>,
    /// Останнє, що лежить у NVS, щоб не писати у флеш те саме.
    persisted: String,
}

impl Inner {
    fn persist(&mut self) {
        let s = self.queue.serialize();
        if s == self.persisted {
            return;
        }
        match self.nvs.set_str(NVS_QUEUE_KEY, &s) {
            Ok(()) => self.persisted = s,
            Err(e) => warn!("NVS: не вдалося зберегти чергу: {e}"),
        }
    }
}

pub struct Shared {
    inner: Mutex<Inner>,
    /// Кеш робочого часу; оновлює цикл заліза.
    working_time: AtomicBool,
    /// Цикл заліза просить бот сповістити нового першого в черзі.
    notify_next: AtomicBool,
    /// Бот просить цикл заліза пікнути (buzzer живе в одному потоці).
    beep: AtomicU8,
}

impl Shared {
    /// Відкриває NVS і відновлює чергу після перезавантаження.
    pub fn new(partition: EspDefaultNvsPartition) -> anyhow::Result<Self> {
        let nvs = EspNvs::new(partition, NVS_NAMESPACE, true)?;
        let stored = match nvs.str_len(NVS_QUEUE_KEY)? {
            Some(len) => {
                let mut buf = vec![0; len + 1];
                nvs.get_str(NVS_QUEUE_KEY, &mut buf)?.unwrap_or_default().to_owned()
            }
            None => String::new(),
        };
        let queue = Queue::deserialize(&stored);
        info!("NVS: відновлено {} записів черги", queue.len());
        Ok(Self {
            inner: Mutex::new(Inner { queue, water: [None, None], nvs, persisted: stored }),
            working_time: AtomicBool::new(true),
            notify_next: AtomicBool::new(false),
            beep: AtomicU8::new(BEEP_NONE),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // panic = abort, тож отруєного м'ютекса не буває.
        self.inner.lock().unwrap()
    }

    /// Змінює чергу й зберігає її в NVS, якщо вона змінилась.
    pub fn modify<R>(&self, f: impl FnOnce(&mut Queue) -> R) -> R {
        let mut inner = self.lock();
        let r = f(&mut inner.queue);
        inner.persist();
        r
    }

    pub fn read<R>(&self, f: impl FnOnce(&Queue) -> R) -> R {
        f(&self.lock().queue)
    }

    pub fn head(&self) -> Option<Member> {
        self.read(|q| q.head().cloned())
    }

    /// `None` — неробочий час.
    pub fn try_join(&self, user_id: i64, name: &str) -> Option<JoinResult> {
        if !self.is_working_time() {
            return None;
        }
        Some(self.modify(|q| q.try_join(user_id, name)))
    }

    pub fn set_water(&self, shower: usize, water: WaterTemperature) {
        self.lock().water[shower] = Some(water);
    }

    pub fn info_report(&self) -> String {
        let inner = self.lock();
        texts::info_report([inner.water[0].as_ref(), inner.water[1].as_ref()], inner.queue.len())
    }

    pub fn is_working_time(&self) -> bool {
        self.working_time.load(Ordering::Relaxed)
    }

    pub fn set_working_time(&self, working: bool) {
        self.working_time.store(working, Ordering::Relaxed);
    }

    pub fn request_notify_next(&self) {
        self.notify_next.store(true, Ordering::Release);
    }

    pub fn take_notify_next(&self) -> bool {
        self.notify_next.swap(false, Ordering::Acquire)
    }

    pub fn request_beep(&self, is_error: bool) {
        self.beep.store(if is_error { BEEP_ERROR } else { BEEP_OK }, Ordering::Release);
    }

    /// `Some(is_error)`, якщо бот просив пікнути.
    pub fn take_beep(&self) -> Option<bool> {
        match self.beep.swap(BEEP_NONE, Ordering::Acquire) {
            BEEP_NONE => None,
            b => Some(b == BEEP_ERROR),
        }
    }
}
