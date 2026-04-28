use exn::{Exn, ResultExt};
use newtype_ids::IntegerIdentifier as _;

use crate::{
    community::Community,
    community_repo::CommunityProvider,
    error::{DbError, Error, StorageLayerError},
    event_log::{Effect, EventPayload, SequenceId, StateMutation},
    event_log_repo::{EventLogProvider, EventLogRepo},
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

impl<E, ELR, CP, G> Providence<ELR, CP, G>
where
    E: DbError,
    ELR: EventLogRepo + EventLogProvider<Error = E>,
    CP: CommunityProvider<Error = E>,
    G: Granter,
{
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
    /// **Idempotent retry**: if the most recent Grant event for this community
    /// has no corresponding effect (e.g. because a previous call crashed between
    /// appending the event and writing the effect), this method resumes that
    /// orphaned grant rather than appending a new one. Callers can safely retry
    /// `grant_fruit` on failure without producing duplicate grant events.
    ///
    /// `&mut self` is required because [`Granter::grant`] takes `&mut self`.
    ///
    /// Order of operations:
    /// 1. Fetch the two most recent grants. If the latest has no effect, resume
    ///    it; otherwise append a new Grant event.
    /// 2. Fetch records between the previous and current grants.
    /// 3. Compute luck mutations.
    /// 4. Apply luck mutations to a community snapshot.
    /// 5. Call `granter.grant` on the adjusted snapshot.
    /// 6. Persist the combined effect.
    pub fn grant_fruit(
        &mut self,
        community: &Community,
        count: usize,
    ) -> Result<Vec<StateMutation>, Exn<Error>> {
        let recent_grants = self
            .event_log
            .get_latest_grant_events(community.id, 2)
            .map_err(|e| {
                let msg = "failed to read grant events to check for in-progress grant";
                StorageLayerError::raise(msg, e)
            })?;

        let latest_is_orphaned = match recent_grants.first() {
            Some(e) => self
                .event_log
                .get_effect_for_event(e.community_id, e.id)
                .map_err(|e| {
                    let msg =
                        "failed to read effect while testing whether latest grant is in orphaned";
                    StorageLayerError::raise(msg, e)
                })?
                .is_none(),
            None => false,
        };

        let (grant_event, prev_grant_id) = if latest_is_orphaned {
            let prev = recent_grants
                .get(1)
                .map(|e| e.id)
                .unwrap_or_else(SequenceId::zero);
            (recent_grants[0].clone(), prev)
        } else {
            let prev = recent_grants
                .first()
                .map(|e| e.id)
                .unwrap_or_else(SequenceId::zero);
            let event = self
                .event_log
                .append_event(community.id, EventPayload::Grant { count })
                .map_err(|e| StorageLayerError::raise("failed to create grant event", e))?;
            (event, prev)
        };

        // If we made it this far, then the grant event has been persisted and
        // retries are guaranteed to be safe

        let community_at_last_grant = self
            .community_provider
            .get(community.id, prev_grant_id)
            .or_raise(|| {
                Error::GrantInterrupted(
                    "failed to read latest community while processing grant".to_string(),
                )
            })?
            .unwrap_or_else(|| Community::new().with_id(community.id));

        let records_since_last_grant = self
            .event_log
            .get_records_between(community.id, prev_grant_id, grant_event.id)
            .or_raise(|| {
                Error::GrantInterrupted("failed to read records between grants".to_string())
            })?;

        let recent_gift_records = self
            .event_log
            .get_latest_gift_records(community.id, 100)
            .or_raise(|| {
                Error::GrantInterrupted(
                    "failed to read gift history while processing grant".to_string(),
                )
            })?;

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
            .append_effect(grant_event.id, community.id, all_mutations.clone())
            .or_raise(|| Error::GrantInterrupted("failed to create grant effect".to_string()))?;

        Ok(all_mutations)
    }
}

#[cfg(test)]
#[path = "providence_tests.rs"]
mod tests;
