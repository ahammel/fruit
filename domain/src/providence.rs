use newtype_ids::IntegerIdentifier as _;

use crate::{
    community::Community,
    community_repo::CommunityProvider,
    error::Error,
    event_log::{Effect, EventPayload, SequenceId, StateMutation},
    event_log_repo::EventLogRepo,
    granter::Granter,
    luck_adjustments,
};

/// Top-level entry point for performing a fruit grant.
///
/// Owns the event log repo and community provider so it can persist the grant
/// event and its effect, as well as read the data needed to compute luck adjustments.
pub struct Providence<ELR: EventLogRepo, CP: CommunityProvider, G: Granter> {
    event_log: ELR,
    community_provider: CP,
    granter: G,
}

impl<ELR: EventLogRepo, CP: CommunityProvider, G: Granter> Providence<ELR, CP, G> {
    /// Creates a new `Providence`.
    pub fn new(event_log: ELR, community_provider: CP, granter: G) -> Self {
        Self {
            event_log,
            community_provider,
            granter,
        }
    }

    /// Appends a Grant event, computes and applies luck adjustments, calls the
    /// granter, appends the combined effect, and returns all mutations.
    ///
    /// `&mut self` is required because [`Granter::grant`] takes `&mut self`.
    ///
    /// Order of operations:
    /// 1. Fetch the previous grant to establish the luck-adjustment window.
    /// 2. Append the Grant event (establishing its sequence ID).
    /// 3. Fetch records between the previous and current grants.
    /// 4. Compute luck mutations.
    /// 5. Apply luck mutations to a community snapshot.
    /// 6. Call `granter.grant` on the adjusted snapshot.
    /// 7. Persist the combined effect.
    pub fn grant_fruit(
        &mut self,
        community: &Community,
        count: usize,
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

        let grant_event = self
            .event_log
            .append_event(community.id, EventPayload::Grant { count })?;

        let records_since_last_grant =
            self.event_log
                .get_records_between(community.id, prev_grant_id, grant_event.id)?;

        let recent_gift_records = self.event_log.get_latest_gift_records(community.id, 100)?;

        let luck_mutations = luck_adjustments::compute(
            &community_at_last_grant,
            &records_since_last_grant,
            &recent_gift_records,
        );

        let mut adjusted = community.clone();
        Effect {
            id: grant_event.id,
            community_id: community.id,
            mutations: luck_mutations.clone(),
        }
        .apply(&mut adjusted);

        let fruit_mutations = self.granter.grant(&adjusted, count);

        let all_mutations = [luck_mutations, fruit_mutations].concat();
        self.event_log
            .append_effect(grant_event.id, community.id, all_mutations.clone())?;

        Ok(all_mutations)
    }
}

#[cfg(test)]
#[path = "providence_tests.rs"]
mod tests;
