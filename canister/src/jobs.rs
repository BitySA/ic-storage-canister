use crate::state::mutate_state;
use crate::utils::trace;
use ic_cdk_timers::set_timer_interval;
use std::time::Duration;

/// Run the abandoned-upload GC every hour. Removes init/in-progress entries
/// older than 24h.
const GC_INTERVAL: Duration = Duration::from_secs(60 * 60);
const GC_TTL_NANOS: u64 = 24 * 60 * 60 * 1_000_000_000;

pub fn start_jobs() {
    let _ = set_timer_interval(GC_INTERVAL, || async move {
        let now = ic_cdk::api::time();
        let removed =
            mutate_state(|state| state.data.storage.gc_abandoned_uploads(now, GC_TTL_NANOS));
        if removed > 0 {
            trace(&format!(
                "abandoned-upload GC: removed {removed} stale entries"
            ));
        }
    });
}
