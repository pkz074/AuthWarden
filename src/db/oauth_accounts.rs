use sqlx::{Error as SqlxError, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::oauth_account::{NewOAuthAccount, OAuthAccount},
};

pub async fn find_oauth_account_by_provider_id(
    pool: &PgPool,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OAuthAccount>, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        SELECT id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        FROM oauth_accounts
        WHERE provider = $1 AND provider_user_id = $2
        "#,
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(account)
}

pub async fn find_oauth_account_by_provider_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OAuthAccount>, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        SELECT id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        FROM oauth_accounts
        WHERE provider = $1 AND provider_user_id = $2
        "#,
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(account)
}

pub async fn find_oauth_accounts_by_user_id(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<OAuthAccount>, AppError> {
    let accounts = sqlx::query_as::<_, OAuthAccount>(
        r#"
        SELECT id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        FROM oauth_accounts
        WHERE user_id = $1
        ORDER BY provider
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(accounts)
}

pub async fn create_oauth_account(
    pool: &PgPool,
    new_account: NewOAuthAccount,
) -> Result<OAuthAccount, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        INSERT INTO oauth_accounts (user_id, provider, provider_user_id, provider_email)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        "#,
    )
    .bind(new_account.user_id)
    .bind(new_account.provider)
    .bind(new_account.provider_user_id)
    .bind(new_account.provider_email)
    .fetch_one(pool)
    .await
    .map_err(map_create_oauth_account_error)?;

    Ok(account)
}

pub async fn create_oauth_account_tx(
    tx: &mut Transaction<'_, Postgres>,
    new_account: NewOAuthAccount,
) -> Result<OAuthAccount, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        INSERT INTO oauth_accounts (user_id, provider, provider_user_id, provider_email)
        VALUES ($1, $2, $3, $4)
        RETURNING id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        "#,
    )
    .bind(new_account.user_id)
    .bind(new_account.provider)
    .bind(new_account.provider_user_id)
    .bind(new_account.provider_email)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_create_oauth_account_error)?;

    Ok(account)
}

pub async fn update_oauth_account_email(
    pool: &PgPool,
    account_id: Uuid,
    provider_email: Option<String>,
) -> Result<OAuthAccount, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        UPDATE oauth_accounts
        SET provider_email = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        "#,
    )
    .bind(account_id)
    .bind(provider_email)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(account)
}

pub async fn update_oauth_account_email_tx(
    tx: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
    provider_email: Option<String>,
) -> Result<OAuthAccount, AppError> {
    let account = sqlx::query_as::<_, OAuthAccount>(
        r#"
        UPDATE oauth_accounts
        SET provider_email = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING id, user_id, provider, provider_user_id, provider_email, created_at, updated_at
        "#,
    )
    .bind(account_id)
    .bind(provider_email)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| AppError::InternalServerError)?;

    Ok(account)
}

fn map_create_oauth_account_error(error: SqlxError) -> AppError {
    if let SqlxError::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
    {
        return AppError::Conflict("oauth account is already linked".to_string());
    }

    AppError::InternalServerError
}
