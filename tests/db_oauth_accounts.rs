mod support;

use authwarden::{db, models::oauth_account::NewOAuthAccount};
use uuid::Uuid;

use support::{test_db, unique_email};

#[tokio::test]
#[ignore = "requires Docker Postgres"]
async fn oauth_account_db_helpers_create_find_update_and_reject_duplicates() {
    let db = test_db().await;
    let email = unique_email("oauth-db");
    let user = db::users::create_oauth_user(&db, email.clone())
        .await
        .unwrap();
    let provider_user_id = format!("provider-user-{}", Uuid::new_v4());

    let account = db::oauth_accounts::create_oauth_account(
        &db,
        NewOAuthAccount {
            user_id: user.id,
            provider: "github".to_string(),
            provider_user_id: provider_user_id.clone(),
            provider_email: Some(email.clone()),
        },
    )
    .await
    .unwrap();

    let found =
        db::oauth_accounts::find_oauth_account_by_provider_id(&db, "github", &provider_user_id)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(found.id, account.id);

    let accounts = db::oauth_accounts::find_oauth_accounts_by_user_id(&db, user.id)
        .await
        .unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].provider, "github");

    let updated_email = format!("updated-{email}");
    let updated = db::oauth_accounts::update_oauth_account_email(
        &db,
        account.id,
        Some(updated_email.clone()),
    )
    .await
    .unwrap();
    assert_eq!(
        updated.provider_email.as_deref(),
        Some(updated_email.as_str())
    );

    let duplicate = db::oauth_accounts::create_oauth_account(
        &db,
        NewOAuthAccount {
            user_id: user.id,
            provider: "github".to_string(),
            provider_user_id,
            provider_email: Some(email),
        },
    )
    .await;
    assert!(duplicate.is_err());
}
