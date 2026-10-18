//! Стан, спільний для циклу заліза і бот-потоків.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use log::{info, warn};
use shower_core::queue::{JoinResult, Queue};
use shower_core::schedule::Mode;
use shower_core::shower::WaterTemperature;
use shower_core::stats::ShowerStats;
use shower_core::texts::{self, StatusView};

use crate::clock::now_ms;
use crate::outbox::Job;

const NVS_NAMESPACE: &str = "ss";
const NVS_QUEUE_KEY: &str = "queue";
const NVS_AVG_KEY: &str = "avg_ms";
const NVS_STATUS_MSG_KEY: &str = "status_msg";

const BEEP_NONE: u8 = 0;
const BEEP_OK: u8 = 1;
const BEEP_ERROR: u8 = 2;

/// Черга закрита (ніч).
#[derive(Debug)]
pub struct Closed;

struct Inner {
    queue: Queue,
    mode: Mode,
    water: [Option<WaterTemperature>; 2],
    /// Коли зайняли кабінку (мс від старту); `None` — вільна.
    busy_since: [Option<u64>; 2],
    stats: ShowerStats,
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

    fn busy_for(&self, now: u64) -> [Option<u64>; 2] {
        self.busy_since.map(|s| s.map(|t| now.saturating_sub(t)))
    }

    fn eta(&self, index: usize, now: u64) -> u64 {
        self.stats.eta_ms(index, &self.busy_for(now))
    }
}

pub struct Shared {
    inner: Mutex<Inner>,
    /// Щось змінилось — статус-повідомлення варто оновити.
    status_dirty: AtomicBool,
    /// Бот просить цикл заліза пікнути (buzzer живе в одному потоці).
    beep: AtomicU8,
    outbox: Sender<Job>,
}

impl Shared {
    /// Відкриває NVS і відновлює чергу та статистику після перезавантаження.
    pub fn new(partition: EspDefaultNvsPartition, outbox: Sender<Job>) -> anyhow::Result<Self> {
        let nvs = EspNvs::new(partition, NVS_NAMESPACE, true)?;
        let stored = match nvs.str_len(NVS_QUEUE_KEY)? {
            Some(len) => {
                let mut buf = vec![0; len + 1];
                nvs.get_str(NVS_QUEUE_KEY, &mut buf)?.unwrap_or_default().to_owned()
            }
            None => String::new(),
        };
        let queue = Queue::deserialize(&stored);
        let stats = nvs.get_u64(NVS_AVG_KEY)?.map(ShowerStats::new).unwrap_or_default();
        info!("NVS: черга {} записів, середній душ {} хв", queue.len(), stats.avg_ms() / 60_000);
        Ok(Self {
            inner: Mutex::new(Inner {
                queue,
                mode: Mode::Open,
                water: [None, None],
                busy_since: [None, None],
                stats,
                nvs,
                persisted: stored,
            }),
            status_dirty: AtomicBool::new(true),
            beep: AtomicU8::new(BEEP_NONE),
            outbox,
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
        drop(inner);
        self.mark_dirty();
        r
    }

    pub fn read<R>(&self, f: impl FnOnce(&Queue) -> R) -> R {
        f(&self.lock().queue)
    }

    pub fn mode(&self) -> Mode {
        self.lock().mode
    }

    pub fn set_mode(&self, mode: Mode) {
        self.lock().mode = mode;
        self.mark_dirty();
    }

    pub fn try_join(&self, user_id: i64, name: &str) -> Result<(JoinResult, u64), Closed> {
        let (result, eta) = {
            let mut inner = self.lock();
            if matches!(inner.mode, Mode::Night { .. }) {
                return Err(Closed);
            }
            let result = inner.queue.try_join(user_id, name);
            inner.persist();
            let eta = match result {
                JoinResult::Added(pos) | JoinResult::AlreadyIn(pos) => inner.eta(pos - 1, now_ms()),
                JoinResult::Full => 0,
            };
            (result, eta)
        };
        self.mark_dirty();
        Ok((result, eta))
    }

    /// `Ok(None)` — черга повна.
    pub fn take_ticket(&self) -> Result<Option<u16>, Closed> {
        if matches!(self.mode(), Mode::Night { .. }) {
            return Err(Closed);
        }
        Ok(self.modify(Queue::take_ticket))
    }

    /// 0-based позиція, довжина черги й очікування.
    pub fn position(&self, user_id: i64) -> (Option<usize>, usize, u64) {
        let inner = self.lock();
        let index = inner.queue.index_of(user_id);
        let eta = index.map_or(0, |i| inner.eta(i, now_ms()));
        (index, inner.queue.len(), eta)
    }

    pub fn status_text(&self) -> String {
        let inner = self.lock();
        let now = now_ms();
        let etas: Vec<u64> = (0..inner.queue.len()).map(|i| inner.eta(i, now)).collect();
        texts::status(&StatusView {
            mode: inner.mode,
            busy: inner.busy_since.map(|s| s.is_some()),
            queue: &inner.queue,
            etas: &etas,
        })
    }

    pub fn shower_occupied(&self, shower: usize, now: u64) {
        self.lock().busy_since[shower] = Some(now);
        self.mark_dirty();
    }

    /// Кабінка звільнилась: враховуємо тривалість у середньому.
    pub fn shower_freed(&self, shower: usize, now: u64) {
        let mut inner = self.lock();
        if let Some(since) = inner.busy_since[shower].take() {
            if inner.stats.record(now - since) {
                let avg = inner.stats.avg_ms();
                if let Err(e) = inner.nvs.set_u64(NVS_AVG_KEY, avg) {
                    warn!("NVS: не вдалося зберегти середній час: {e}");
                }
            }
        }
        drop(inner);
        self.mark_dirty();
    }

    pub fn set_water(&self, shower: usize, water: WaterTemperature) {
        self.lock().water[shower] = Some(water);
    }

    pub fn info_report(&self) -> String {
        let inner = self.lock();
        texts::info_report([inner.water[0].as_ref(), inner.water[1].as_ref()], inner.queue.len())
    }

    pub fn status_message_id(&self) -> Option<i64> {
        self.lock().nvs.get_i64(NVS_STATUS_MSG_KEY).ok().flatten()
    }

    pub fn save_status_message_id(&self, id: i64) {
        if let Err(e) = self.lock().nvs.set_i64(NVS_STATUS_MSG_KEY, id) {
            warn!("NVS: не вдалося зберегти id статус-повідомлення: {e}");
        }
    }

    pub fn mark_dirty(&self) {
        self.status_dirty.store(true, Ordering::Release);
    }

    pub fn take_dirty(&self) -> bool {
        self.status_dirty.swap(false, Ordering::Acquire)
    }

    /// Ставить завдання для потоку відправки.
    pub fn send(&self, job: Job) {
        // Потік відправки живе вічно, тож канал не закривається.
        let _ = self.outbox.send(job);
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
