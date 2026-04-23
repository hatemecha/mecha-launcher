use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct LaunchTracker {
    active_launch_id: Arc<Mutex<Option<String>>>,
}

impl LaunchTracker {
    pub fn try_acquire(&self, launch_id: String) -> bool {
        let mut active_launch = self
            .active_launch_id
            .lock()
            .expect("launch tracker mutex poisoned");

        if active_launch.is_some() {
            return false;
        }

        *active_launch = Some(launch_id);
        true
    }

    pub fn clear_if_matches(&self, launch_id: &str) {
        let mut active_launch = self
            .active_launch_id
            .lock()
            .expect("launch tracker mutex poisoned");

        if active_launch.as_deref() == Some(launch_id) {
            *active_launch = None;
        }
    }
}
