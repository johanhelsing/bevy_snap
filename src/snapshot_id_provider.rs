use std::marker::PhantomData;

use crate::{SnapType, SnapshotId};

/// Provides SnapshotId components
#[derive(Default)]
pub struct SnapshotIdProvider<T: SnapType> {
    next_id: u32,
    t: PhantomData<T>,
}

impl<T: SnapType> SnapshotIdProvider<T> {
    /// Returns an unused, unique id.
    pub fn next(&mut self) -> SnapshotId<T> {
        if self.next_id == u32::MAX {
            // TODO: do something smart?
            panic!("SnapshotIdProvider: u32::MAX has been reached.");
        }
        let id = self.next_id;
        self.next_id += 1;
        SnapshotId::new(id)
    }
}
