use crate::{community::Community, event_log::StateMutation};

/// Port for distributing fruits to all members of a [`Community`].
pub trait Granter {
    /// Compute `count` fruit distributions for each member of `community` and return
    /// the resulting state mutations. The community is not modified.
    ///
    /// `&mut self` is required to accommodate stateful implementations such as
    /// [`RandomGranter`][crate::random_granter::RandomGranter], which advances an
    /// internal RNG on every call.
    fn grant(&mut self, community: &Community, count: usize) -> Vec<StateMutation>;
}
