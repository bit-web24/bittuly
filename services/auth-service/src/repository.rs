use crate::models::{CreateUserPayload, UpdateUserPayload, User};
use crate::repo_trait::UserRepo;
use async_trait::async_trait;
use shared::postgres::DbPool;
use uuid::Uuid;

pub struct PgUserRepo(pub DbPool);

pub struct SplitUserRepo {
    pub primary: std::sync::Arc<dyn UserRepo>,
    pub replica: std::sync::Arc<dyn UserRepo>,
}

#[async_trait]
impl UserRepo for SplitUserRepo {
    async fn create_user(&self, payload: CreateUserPayload) -> Result<User, sqlx::Error> {
        self.primary.create_user(payload).await
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        self.replica.get_user_by_id(user_id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        self.replica.get_user_by_email(email).await
    }

    async fn update_user(
        &self,
        user_id: Uuid,
        payload: UpdateUserPayload,
    ) -> Result<User, sqlx::Error> {
        self.primary.update_user(user_id, payload).await
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        self.primary.delete_user(user_id).await
    }

    async fn ping(&self) -> Result<(), sqlx::Error> {
        self.primary.ping().await?;
        // Ignore replica ping failures to keep the primary up?
        // Usually you'd check both, or just primary. 
        self.replica.ping().await.ok();
        Ok(())
    }
}

#[async_trait]
impl UserRepo for PgUserRepo {
    async fn create_user(&self, payload: CreateUserPayload) -> Result<User, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO users (id, username, email, password)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, password, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&payload.password)
        .fetch_one(&self.0)
        .await
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, username, email, password, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.0)
        .await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as(
            "SELECT id, username, email, password, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.0)
        .await
    }

    async fn update_user(
        &self,
        user_id: Uuid,
        payload: UpdateUserPayload,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE users
            SET
                username = COALESCE($1, username),
                email = COALESCE($2, email),
                password = COALESCE($3, password),
                updated_at = NOW()
            WHERE id = $4
            RETURNING id, username, email, password, created_at, updated_at
            "#,
        )
        .bind(payload.username)
        .bind(payload.email)
        .bind(payload.password)
        .bind(user_id)
        .fetch_one(&self.0)
        .await
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.0).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    // A helper to initialize the schema since we aren't using standard sqlx migrations.
    async fn init_schema(pool: &PgPool) {
        let schema = include_str!("../../../docker/postgres-auth/init/01-init.sql");
        for statement in schema.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt)
                    .execute(pool)
                    .await
                    .expect("failed to execute schema statement");
            }
        }
    }

    #[sqlx::test(migrations = false)]
    async fn test_create_and_get_user(pool: PgPool) {
        init_schema(&pool).await;
        let repo = PgUserRepo(pool);

        let payload = CreateUserPayload {
            username: "db_test".into(),
            email: "db@example.com".into(),
            password: "hashed_pass".into(),
        };

        // Test create
        let user = repo.create_user(payload).await.unwrap();
        assert_eq!(user.username, "db_test");
        assert_eq!(user.email, "db@example.com");

        // Test get by email
        let fetched = repo
            .get_user_by_email("db@example.com")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.id, user.id);

        // Test get by id
        let fetched_by_id = repo.get_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(fetched_by_id.email, "db@example.com");
    }

    #[sqlx::test(migrations = false)]
    async fn test_duplicate_email_error(pool: PgPool) {
        init_schema(&pool).await;
        let repo = PgUserRepo(pool);

        let payload1 = CreateUserPayload {
            username: "user1".into(),
            email: "dup@example.com".into(),
            password: "p1".into(),
        };
        repo.create_user(payload1).await.unwrap();

        let payload2 = CreateUserPayload {
            username: "user2".into(),
            email: "dup@example.com".into(), // Same email
            password: "p2".into(),
        };

        let err = repo.create_user(payload2).await.unwrap_err();
        assert!(matches!(err, sqlx::Error::Database(e) if e.code().as_deref() == Some("23505"))); // unique_violation
    }

    #[sqlx::test(migrations = false)]
    async fn test_update_user(pool: PgPool) {
        init_schema(&pool).await;
        let repo = PgUserRepo(pool);

        let user = repo
            .create_user(CreateUserPayload {
                username: "old".into(),
                email: "old@example.com".into(),
                password: "pass".into(),
            })
            .await
            .unwrap();

        let updated = repo
            .update_user(
                user.id,
                UpdateUserPayload {
                    username: Some("new".into()),
                    email: None, // keep old
                    password: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.username, "new");
        assert_eq!(updated.email, "old@example.com"); // remains unchanged
    }

    #[sqlx::test(migrations = false)]
    async fn test_delete_user(pool: PgPool) {
        init_schema(&pool).await;
        let repo = PgUserRepo(pool);

        let user = repo
            .create_user(CreateUserPayload {
                username: "delete_me".into(),
                email: "del@example.com".into(),
                password: "pass".into(),
            })
            .await
            .unwrap();

        repo.delete_user(user.id).await.unwrap();

        let fetched = repo.get_user_by_id(user.id).await.unwrap();
        assert!(fetched.is_none());
    }
}
