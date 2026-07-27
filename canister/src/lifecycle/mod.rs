pub mod init;
mod post_upgrade;
mod pre_upgrade;
use crate::state::RuntimeState;

pub use init::*;

pub fn init_canister(state: RuntimeState) {
    crate::state::init_state(state);
}

/// Usable bytes a single storage canister will accept before its parent has to
/// spawn another one.
///
/// Test mode is deliberately smaller, but 50 MB was small enough that a staging
/// collection filled a canister after a handful of uploads and had to spawn a
/// sibling, at roughly 1.1 TC each plus the ongoing burn of another canister.
/// Note this is compared against allocated stable memory, which runs ahead of the
/// actual file bytes, so the effective capacity is lower than the number here.
pub fn max_storage_size_for(test_mode: bool) -> u128 {
    if test_mode {
        500 * 1024 * 1024 // 500mb
    } else {
        500 * 1024 * 1024 * 1024 // 500gb
    }
}
