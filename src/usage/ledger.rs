use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Write as _};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Default length of a period: one hour.
pub const DEFAULT_PERIOD: Duration = Duration::from_secs(3600);
/// Default minimum count exported (k-anonymity of the ledger).
pub const DEFAULT_THRESHOLD: u64 = 5;
/// Closed periods kept in memory for the collector to scrape (two days of hours).
pub const DEFAULT_RETAINED_PERIODS: usize = 48;

/// Usage of one operation in one closed period: counts only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEntry {
    pub service: String,
    /// Route template (`/api/v1/users/{id}`), never the path with its values.
    pub operation: String,
    pub method: String,
    pub status: u16,
    /// Unix seconds of the start of the period.
    pub period_start: u64,
    pub actions: u64,
    pub distinct_users: u64,
    pub latency_ms_sum: u64,
}

#[derive(Default)]
struct Counter {
    actions: u64,
    latency_ms_sum: u64,
    // Hashes of the user ids, salted with the period's salt: dropped when the period closes.
    users: HashSet<[u8; 32]>,
}

struct Period {
    index: u64,
    salt: [u8; 32],
    counters: HashMap<(String, String, u16), Counter>,
}

impl Period {
    fn new(index: u64) -> Self {
        let mut salt = [0u8; 32];
        // A failing system RNG leaves a zero salt; the hashes still never leave the process.
        let _ = getrandom::fill(&mut salt);
        Self {
            index,
            salt,
            counters: HashMap::new(),
        }
    }
}

struct State {
    current: Period,
    closed: VecDeque<Vec<UsageEntry>>,
}

/// Usage counters of one service (see the module documentation).
///
/// Thread-safe; share it as `web::Data<UsageLedger>`.
pub struct UsageLedger {
    service: String,
    period: Duration,
    threshold: u64,
    retained: usize,
    state: Mutex<State>,
}

impl fmt::Debug for UsageLedger {
    // Never the salt nor the hashes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UsageLedger")
            .field("service", &self.service)
            .field("period", &self.period)
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

fn period_index(now: SystemTime, period: Duration) -> u64 {
    let secs = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    secs / period.as_secs().max(1)
}

impl UsageLedger {
    /// A ledger for `service` with the defaults: hourly periods, threshold 5, two days kept.
    #[must_use]
    pub fn new(service: &str) -> Self {
        Self::with_settings(
            service,
            DEFAULT_PERIOD,
            DEFAULT_THRESHOLD,
            DEFAULT_RETAINED_PERIODS,
        )
    }

    /// A ledger with an explicit period, threshold `k` and number of closed periods kept.
    #[must_use]
    pub fn with_settings(service: &str, period: Duration, threshold: u64, retained: usize) -> Self {
        let period = if period.as_secs() == 0 {
            DEFAULT_PERIOD
        } else {
            period
        };
        Self {
            service: service.to_string(),
            period,
            threshold: threshold.max(1),
            retained: retained.max(1),
            state: Mutex::new(State {
                current: Period::new(period_index(SystemTime::now(), period)),
                closed: VecDeque::new(),
            }),
        }
    }

    /// The minimum count exported.
    #[must_use]
    pub const fn threshold(&self) -> u64 {
        self.threshold
    }

    // The guard is held until the counter is updated: the salt and the counters must belong to
    // the same period.
    #[allow(clippy::significant_drop_tightening)]
    /// Records one request: its route template, method, status and duration, and, when the
    /// caller is authenticated, its user id (hashed with the period's salt, never kept as is).
    pub fn record(
        &self,
        operation: &str,
        method: &str,
        status: u16,
        latency: Duration,
        user_id: Option<u64>,
        now: SystemTime,
    ) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.roll(&mut state, now);
        let salt = state.current.salt;
        let counter = state
            .current
            .counters
            .entry((operation.to_string(), method.to_string(), status))
            .or_default();
        counter.actions += 1;
        counter.latency_ms_sum += u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
        if let Some(id) = user_id {
            let mut hasher = Sha256::new();
            hasher.update(salt);
            hasher.update(id.to_be_bytes());
            counter.users.insert(hasher.finalize().into());
        }
    }

    /// The closed periods kept, oldest first, with the threshold applied.
    ///
    /// An operation whose actions or distinct users are under `k` is not listed one by one; the
    /// service's small operations are summed into one `other` entry per period, itself dropped
    /// when it is under `k` too.
    #[must_use]
    pub fn closed_entries(&self, now: SystemTime) -> Vec<UsageEntry> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.roll(&mut state, now);
        state.closed.iter().flatten().cloned().collect()
    }

    /// The closed periods in the Prometheus text format, for the collector to scrape.
    #[must_use]
    pub fn render_prometheus(&self, now: SystemTime) -> String {
        let entries = self.closed_entries(now);
        let mut out = String::new();
        out.push_str("# HELP mairie360_usage_actions Requests per operation and period (MAIR-501).\n# TYPE mairie360_usage_actions gauge\n");
        for e in &entries {
            let _ = writeln!(
                out,
                "mairie360_usage_actions{{{}}} {}",
                labels(e),
                e.actions
            );
        }
        out.push_str("# HELP mairie360_usage_distinct_users Distinct users per operation and period, counted without identifiers.\n# TYPE mairie360_usage_distinct_users gauge\n");
        for e in &entries {
            let _ = writeln!(
                out,
                "mairie360_usage_distinct_users{{{}}} {}",
                labels(e),
                e.distinct_users
            );
        }
        out.push_str("# HELP mairie360_usage_latency_ms_sum Summed duration of the requests, in milliseconds.\n# TYPE mairie360_usage_latency_ms_sum gauge\n");
        for e in &entries {
            let _ = writeln!(
                out,
                "mairie360_usage_latency_ms_sum{{{}}} {}",
                labels(e),
                e.latency_ms_sum
            );
        }
        out
    }

    fn roll(&self, state: &mut State, now: SystemTime) {
        let index = period_index(now, self.period);
        if index == state.current.index {
            return;
        }
        let closing = std::mem::replace(&mut state.current, Period::new(index));
        let entries = self.close(closing);
        if !entries.is_empty() {
            state.closed.push_back(entries);
        }
        while state.closed.len() > self.retained {
            state.closed.pop_front();
        }
    }

    // Turns a period into counts and drops its salt and hashes with it.
    fn close(&self, period: Period) -> Vec<UsageEntry> {
        let period_start = period.index * self.period.as_secs();
        let mut entries = Vec::new();
        let mut other = UsageEntry {
            service: self.service.clone(),
            operation: "other".to_string(),
            method: "other".to_string(),
            status: 0,
            period_start,
            actions: 0,
            distinct_users: 0,
            latency_ms_sum: 0,
        };
        let mut other_users: HashSet<[u8; 32]> = HashSet::new();
        let mut keys: Vec<_> = period.counters.keys().cloned().collect();
        keys.sort();
        let mut counters = period.counters;
        for key in keys {
            let Some(counter) = counters.remove(&key) else {
                continue;
            };
            let distinct = counter.users.len() as u64;
            if counter.actions < self.threshold || (distinct > 0 && distinct < self.threshold) {
                other.actions += counter.actions;
                other.latency_ms_sum += counter.latency_ms_sum;
                other_users.extend(counter.users);
                continue;
            }
            entries.push(UsageEntry {
                service: self.service.clone(),
                operation: key.0,
                method: key.1,
                status: key.2,
                period_start,
                actions: counter.actions,
                distinct_users: distinct,
                latency_ms_sum: counter.latency_ms_sum,
            });
        }
        other.distinct_users = other_users.len() as u64;
        if other.actions >= self.threshold
            && (other.distinct_users == 0 || other.distinct_users >= self.threshold)
        {
            entries.push(other);
        }
        entries
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn labels(e: &UsageEntry) -> String {
    format!(
        "service=\"{}\",operation=\"{}\",method=\"{}\",status=\"{}\",period_start=\"{}\"",
        escape(&e.service),
        escape(&e.operation),
        escape(&e.method),
        e.status,
        e.period_start
    )
}

#[cfg(test)]
mod tests {
    use super::{period_index, UsageLedger};
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn the_salt_changes_with_the_period_and_the_hashes_go_with_it() {
        let ledger = UsageLedger::with_settings("core-api", Duration::from_mins(1), 1, 10);
        let t0 = UNIX_EPOCH + Duration::from_mins(10);
        ledger.record("/a", "GET", 200, Duration::from_millis(1), Some(7), t0);
        let (salt0, hashes0) = {
            let state = ledger.state.lock().unwrap();
            (
                state.current.salt,
                state
                    .current
                    .counters
                    .values()
                    .map(|c| c.users.len())
                    .sum::<usize>(),
            )
        };
        assert_eq!(hashes0, 1);
        let t1 = t0 + Duration::from_mins(1);
        ledger.record("/a", "GET", 200, Duration::from_millis(1), Some(7), t1);
        let state = ledger.state.lock().unwrap();
        assert_eq!(
            state.current.index,
            period_index(t1, Duration::from_mins(1))
        );
        assert_ne!(state.current.salt, salt0, "a new salt for every period");
        assert_eq!(
            state.closed.len(),
            1,
            "the previous period is closed: counts only"
        );
        assert_eq!(state.closed[0][0].distinct_users, 1);
    }

    #[test]
    fn debug_never_shows_the_salt() {
        let ledger = UsageLedger::new("core-api");
        let text = format!("{ledger:?}");
        assert!(
            text.contains("core-api") && !text.contains("salt"),
            "{text}"
        );
    }
}
