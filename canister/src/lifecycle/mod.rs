pub mod init;
mod post_upgrade;
mod pre_upgrade;
use crate::state::RuntimeState;

pub use init::*;

pub fn init_canister(state: RuntimeState) {
    crate::state::init_state(state);
}
