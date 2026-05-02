use aws_sdk_dynamodb::{
    operation::{
        get_item::{GetItemError, GetItemOutput},
        put_item::{PutItemError, PutItemOutput},
        query::{QueryError, QueryOutput},
    },
    types::error::{ConditionalCheckFailedException, InternalServerError},
};
use aws_smithy_mocks::{mock, mock_client};
use fruit_domain::community_repo::{CommunityPersistor, CommunityProvider};
use uuid::Uuid;

use crate::{
    community_repo::DynamoDbCommunityRepo, dto::community::encode_community, error::Error,
};

/// A fixed member ID accessible from within the test module.
fn member_id() -> fruit_domain::member::MemberId {
    fruit_domain::member::MemberId::from(Uuid::from_bytes([11u8; 16]))
}

mod test_helpers {
    use fruit_domain::{
        bag::Bag,
        community::{Community, CommunityId},
        event_log::SequenceId,
        fruit::{GRAPES, MELON},
        member::Member,
    };
    use uuid::Uuid;

    /// A fixed community ID for test reproducibility.
    pub fn community_id() -> CommunityId {
        CommunityId::from(Uuid::from_bytes([10u8; 16]))
    }

    /// A fixed member ID for test reproducibility.
    pub fn member_id() -> fruit_domain::member::MemberId {
        super::member_id()
    }

    /// Wraps `n` as a `SequenceId`.
    pub fn seq(n: u64) -> SequenceId {
        SequenceId::new(n)
    }

    /// Builds a test community with one member who holds some fruit.
    pub fn make_community() -> Community {
        let bag = Bag::new().insert(GRAPES).insert(MELON);
        let m = Member::new("Alice")
            .with_id(member_id())
            .with_bag(bag)
            .with_luck(50);
        let mut c = Community::new()
            .with_id(community_id())
            .with_luck(30)
            .with_version(seq(5));
        c.add_member(m);
        c
    }
}

use test_helpers::*;

// ── CommunityProvider::get ────────────────────────────────────────────────────

#[tokio::test]
async fn get_found_returns_decoded_community() {
    let community = make_community();
    let item = encode_community(&community).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::get_item).then_output(move || {
        GetItemOutput::builder()
            .set_item(Some(item.clone()))
            .build()
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get(community_id(), seq(5)).await.unwrap();
    assert_eq!(result, Some(community));
}

#[tokio::test]
async fn get_not_found_returns_none() {
    let rule =
        mock!(aws_sdk_dynamodb::Client::get_item).then_output(|| GetItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get(community_id(), seq(5)).await.unwrap();
    assert_eq!(result, None);
}

// ── CommunityProvider::get_latest ────────────────────────────────────────────

#[tokio::test]
async fn get_latest_found_returns_decoded_community() {
    let community = make_community();
    let item = encode_community(&community).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(item.clone()).build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get_latest(community_id()).await.unwrap();
    assert_eq!(result, Some(community));
}

#[tokio::test]
async fn get_latest_not_found_returns_none() {
    let rule =
        mock!(aws_sdk_dynamodb::Client::query).then_output(|| QueryOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get_latest(community_id()).await.unwrap();
    assert_eq!(result, None);
}

// ── CommunityPersistor::put ───────────────────────────────────────────────────

#[tokio::test]
async fn put_success_returns_community() {
    let community = make_community();

    let rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.put(community.clone()).await.unwrap();
    assert_eq!(result, community);
}

#[tokio::test]
async fn put_conflict_returns_already_exists_error() {
    let community = make_community();

    let rule = mock!(aws_sdk_dynamodb::Client::put_item).then_error(|| {
        PutItemError::ConditionalCheckFailedException(
            ConditionalCheckFailedException::builder().build(),
        )
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.put(community).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::AlreadyExists { .. })),
        "expected AlreadyExists, got {:?}",
        result
    );
}

#[tokio::test]
async fn put_sdk_error_returns_sdk_error() {
    let community = make_community();

    let rule = mock!(aws_sdk_dynamodb::Client::put_item)
        .then_error(|| PutItemError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.put(community).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── CommunityProvider SDK errors ──────────────────────────────────────────────

#[tokio::test]
async fn get_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::get_item)
        .then_error(|| GetItemError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get(community_id(), seq(5)).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

#[tokio::test]
async fn get_latest_sdk_error_returns_sdk_error() {
    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_error(|| QueryError::InternalServerError(InternalServerError::builder().build()));
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let result = repo.get_latest(community_id()).await;
    assert!(
        matches!(result, Err(ref e) if matches!(&**e, Error::Sdk(_))),
        "expected Sdk error, got {:?}",
        result
    );
}

// ── Reference forwarding impls ────────────────────────────────────────────────

#[tokio::test]
async fn get_via_ref() {
    let community = make_community();
    let item = encode_community(&community).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::get_item).then_output(move || {
        GetItemOutput::builder()
            .set_item(Some(item.clone()))
            .build()
    });
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r).get(community_id(), seq(5)).await.unwrap();
    assert_eq!(result, Some(community));
}

#[tokio::test]
async fn get_latest_via_ref() {
    let community = make_community();
    let item = encode_community(&community).unwrap();

    let rule = mock!(aws_sdk_dynamodb::Client::query)
        .then_output(move || QueryOutput::builder().items(item.clone()).build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r).get_latest(community_id()).await.unwrap();
    assert_eq!(result, Some(community));
}

#[tokio::test]
async fn put_via_ref() {
    let community = make_community();

    let rule =
        mock!(aws_sdk_dynamodb::Client::put_item).then_output(|| PutItemOutput::builder().build());
    let client = mock_client!(aws_sdk_dynamodb, [&rule]);
    let repo = DynamoDbCommunityRepo::new(client, "test-table");

    let r = &repo;
    let result = (&r).put(community.clone()).await.unwrap();
    assert_eq!(result, community);
}
