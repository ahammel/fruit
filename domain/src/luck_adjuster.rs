use newtype_ids::IntegerIdentifier as _;

use crate::{
    community::Community,
    community_repo::CommunityProvider,
    error::Error,
    event_log::{SequenceId, StateMutation},
    event_log_repo::EventLogProvider,
    luck_adjustments,
};

/// Computes luck-adjustment mutations for a community at grant time.
///
/// Wraps an [`EventLogProvider`] and a [`CommunityProvider`] to fetch the data
/// needed to call [`luck_adjustments::compute`].
pub struct LuckAdjuster<ELP: EventLogProvider, CP: CommunityProvider> {
    event_log: ELP,
    community_provider: CP,
}

impl<ELP: EventLogProvider, CP: CommunityProvider> LuckAdjuster<ELP, CP> {
    /// Creates a new `LuckAdjuster`.
    pub fn new(event_log: ELP, community_provider: CP) -> Self {
        Self {
            event_log,
            community_provider,
        }
    }

    /// Computes luck mutations for `community` at the point just before `before`.
    ///
    /// 1. Looks up the most recent previous grant to establish the window start.
    /// 2. Fetches the community snapshot at that grant (falling back to a fresh
    ///    community with the same ID if none is stored).
    /// 3. Fetches all records in the window and up to 100 recent gift records.
    /// 4. Delegates to [`luck_adjustments::compute`].
    pub fn compute(
        &self,
        community: &Community,
        before: SequenceId,
    ) -> Result<Vec<StateMutation>, Error> {
        let prev_grant_id = self
            .event_log
            .get_latest_grant_events(community.id, 1)?
            .into_iter()
            .next()
            .map(|e| e.id)
            .unwrap_or_else(SequenceId::zero);

        let community_at_last_grant = self
            .community_provider
            .get(community.id, prev_grant_id)?
            .unwrap_or_else(|| Community::new().with_id(community.id));

        let records_since_last_grant =
            self.event_log
                .get_records_between(community.id, prev_grant_id, before)?;

        let recent_gift_records = self.event_log.get_latest_gift_records(community.id, 100)?;

        Ok(luck_adjustments::compute(
            &community_at_last_grant,
            &records_since_last_grant,
            &recent_gift_records,
        ))
    }
}

#[cfg(test)]
#[path = "luck_adjuster_tests.rs"]
mod tests;
