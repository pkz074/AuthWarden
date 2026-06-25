use sqlx::PgPool;

use crate::{
    errors::AppError,
    models::user::{NewUser, User},
};

pub async fn create_user(pool: &PgPool, new_user: NewUser) -> Result<User, AppError> {
    // TODO

    let _ = (pool, new_user);

    Err(AppError::InternalServerError)
}
