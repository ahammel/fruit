use crate::community::Community;

/// Port for distributing fruits to all members of a [`Community`].
pub trait Granter {
    /// Grants `count` fruits to each member of `community`.
    fn grant(&mut self, community: &mut Community, count: usize);
}
