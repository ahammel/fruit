use crate::{
    community::CommunityId,
    effect::Effect,
    event::{Event, SequenceId},
};

/// A single entry in the shared event/effect log, identified by a [`SequenceId`](crate::event::SequenceId).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// A player intention that was recorded.
    EventRecord(Event),
    /// The computed consequences of an event.
    EffectRecord(Effect),
}

impl Record {
    pub fn id(&self) -> SequenceId {
        match self {
            Record::EventRecord(e) => e.id,
            Record::EffectRecord(e) => e.id,
        }
    }

    pub fn community_id(&self) -> CommunityId {
        match self {
            Record::EventRecord(e) => e.community_id,
            Record::EffectRecord(e) => e.community_id,
        }
    }
}

impl From<Event> for Record {
    fn from(value: Event) -> Self {
        Record::EventRecord(value)
    }
}

impl From<Effect> for Record {
    fn from(value: Effect) -> Self {
        Record::EffectRecord(value)
    }
}
