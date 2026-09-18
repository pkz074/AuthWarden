use sqlx::PgPool;

use crate::{
    db,
    errors::AppError,
    models::{oauth_account::NewOAuthAccount, user::User},
};

pub async fn find_or_create_oauth_user(
    pool: &PgPool,
    provider: &str,
    provider_user_id: String,
    provider_email: String,
) -> Result<User, AppError> {
    let email = normalize_email(&provider_email);
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if let Some(account) = db::oauth_accounts::find_oauth_account_by_provider_id_tx(
        &mut tx,
        provider,
        &provider_user_id,
    )
    .await?
    {
        db::oauth_accounts::update_oauth_account_email_tx(&mut tx, account.id, Some(email)).await?;

        let user = db::users::find_user_by_id_tx(&mut tx, account.user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        tx.commit()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        return Ok(user);
    }

    let user = match db::users::find_user_by_email_tx(&mut tx, &email).await? {
        Some(user) => user,
        None => db::users::create_oauth_user_tx(&mut tx, email.clone()).await?,
    };

    let new_account = NewOAuthAccount {
        user_id: user.id,
        provider: provider.to_string(),
        provider_user_id,
        provider_email: Some(email),
    };

    db::oauth_accounts::create_oauth_account_tx(&mut tx, new_account).await?;

    tx.commit()
        .await
        .map_err(|_| AppError::InternalServerError)?;

    Ok(user)
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
