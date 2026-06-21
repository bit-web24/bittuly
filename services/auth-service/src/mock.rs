#![allow(clippy::new_without_default)]
use crate::models::{CreateUserPayload, UpdateUserPayload, User};
use crate::repo_trait::UserRepo;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

pub struct MockUserRepo {
    users: RwLock<HashMap<Uuid, User>>,
}

impl MockUserRepo {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl UserRepo for MockUserRepo {
    async fn create_user(&self, payload: CreateUserPayload) -> Result<User, sqlx::Error> {
        let mut users = self.users.write().unwrap();
        let user = User {
            id: Uuid::new_v4(),
            username: payload.username,
            email: payload.email,
            password: payload.password,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.read().unwrap();
        Ok(users.get(&user_id).cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let users = self.users.read().unwrap();
        Ok(users.values().find(|u| u.email == email).cloned())
    }

    async fn update_user(
        &self,
        user_id: Uuid,
        payload: UpdateUserPayload,
    ) -> Result<User, sqlx::Error> {
        let mut users = self.users.write().unwrap();
        if let Some(user) = users.get_mut(&user_id) {
            if let Some(username) = payload.username {
                user.username = username;
            }
            if let Some(email) = payload.email {
                user.email = email;
            }
            if let Some(password) = payload.password {
                user.password = password;
            }
            user.updated_at = chrono::Utc::now();
            Ok(user.clone())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        self.users.write().unwrap().remove(&user_id);
        Ok(())
    }

    async fn ping(&self) -> Result<(), sqlx::Error> {
        Ok(())
    }
}
