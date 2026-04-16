//! Note entity and service.

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::persistence::{NoteRepository, RepositoryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    id: Uuid,
    user_id: String,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Note {
    #[must_use]
    pub fn new(user_id: String, title: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            title,
            content,
            created_at: now,
            updated_at: now,
        }
    }

    #[must_use]
    pub fn from_parts(
        id: Uuid,
        user_id: String,
        title: String,
        content: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            title,
            content,
            created_at,
            updated_at,
        }
    }

    #[must_use]
    pub fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.updated_at = Utc::now();
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Error)]
pub enum NoteServiceError {
    #[error("Note not found")]
    NotFound,
    #[error("Access denied")]
    Forbidden,
    #[error("Failed to process note: {0}")]
    Internal(String),
}

impl From<RepositoryError> for NoteServiceError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::NotFound => Self::NotFound,
            RepositoryError::Forbidden => Self::Forbidden,
            RepositoryError::Storage(msg) => Self::Internal(msg),
        }
    }
}

/// Generic service that accepts any `NoteRepository` implementation.
#[derive(Debug, Clone)]
pub struct NoteService<R: NoteRepository> {
    repository: R,
}

impl<R: NoteRepository> NoteService<R> {
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_notes(&self, user_id: &str) -> Result<Vec<Note>, NoteServiceError> {
        tracing::debug!(user_id = %user_id, "Listing notes");
        let notes = self.repository.list(user_id).await?;
        tracing::debug!(user_id = %user_id, count = notes.len(), "Found notes");
        Ok(notes)
    }

    pub async fn get_note(&self, user_id: &str, note_id: Uuid) -> Result<Note, NoteServiceError> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Getting note");
        Ok(self.repository.get(user_id, note_id).await?)
    }

    pub async fn create_note(
        &self,
        user_id: &str,
        title: String,
        content: String,
    ) -> Result<Note, NoteServiceError> {
        tracing::debug!(user_id = %user_id, title = %title, "Creating note");
        let note = Note::new(user_id.to_string(), title, content);
        let created = self.repository.create(note).await?;
        tracing::info!(user_id = %user_id, note_id = %created.id(), "Note created");
        Ok(created)
    }

    pub async fn update_note(
        &self,
        user_id: &str,
        note_id: Uuid,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Note, NoteServiceError> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Updating note");

        let mut note = self.repository.get(user_id, note_id).await?;
        if let Some(t) = title {
            note.set_title(t);
        }
        if let Some(c) = content {
            note.set_content(c);
        }
        let updated = self.repository.update(user_id, note).await?;

        tracing::info!(user_id = %user_id, note_id = %note_id, "Note updated");
        Ok(updated)
    }

    pub async fn delete_note(&self, user_id: &str, note_id: Uuid) -> Result<(), NoteServiceError> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Deleting note");
        self.repository.delete(user_id, note_id).await?;
        tracing::info!(user_id = %user_id, note_id = %note_id, "Note deleted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::mock::MockNoteRepository;

    fn create_test_service() -> NoteService<MockNoteRepository> {
        NoteService::new(MockNoteRepository::new())
    }

    mod note_entity {
        use super::*;

        #[test]
        fn it_should_generate_id_and_timestamps_on_new() {
            let note = Note::new(
                "user-1".to_string(),
                "Title".to_string(),
                "Content".to_string(),
            );
            assert!(!note.id().is_nil());
            assert_eq!(note.user_id(), "user-1");
            assert_eq!(note.created_at(), note.updated_at());
        }

        #[test]
        fn it_should_update_timestamp_when_setting_title() {
            let mut note = Note::new(
                "user-1".to_string(),
                "Old".to_string(),
                "Content".to_string(),
            );
            let original = note.updated_at();
            std::thread::sleep(std::time::Duration::from_millis(10));
            note.set_title("New".to_string());
            assert_eq!(note.title(), "New");
            assert!(note.updated_at() > original);
        }

        #[test]
        fn it_should_update_timestamp_when_setting_content() {
            let mut note = Note::new("user-1".to_string(), "Title".to_string(), "Old".to_string());
            let original = note.updated_at();
            std::thread::sleep(std::time::Duration::from_millis(10));
            note.set_content("New".to_string());
            assert_eq!(note.content(), "New");
            assert!(note.updated_at() > original);
        }
    }

    mod note_service {
        use super::*;

        #[tokio::test]
        async fn it_should_list_notes_for_user() {
            let mut service = create_test_service();
            let notes = vec![
                Note::new(
                    "user-1".to_string(),
                    "Note 1".to_string(),
                    "Content".to_string(),
                ),
                Note::new(
                    "user-1".to_string(),
                    "Note 2".to_string(),
                    "Content".to_string(),
                ),
            ];
            let cloned = notes.clone();

            service
                .repository
                .expect_list()
                .withf(|uid| uid == "user-1")
                .times(1)
                .return_once(move |_| Ok(cloned));

            let result = service.list_notes("user-1").await.unwrap();
            assert_eq!(result.len(), 2);
        }

        #[tokio::test]
        async fn it_should_get_note_by_id() {
            let mut service = create_test_service();
            let note = Note::new(
                "user-1".to_string(),
                "Title".to_string(),
                "Content".to_string(),
            );
            let note_id = note.id();
            let cloned = note.clone();

            service
                .repository
                .expect_get()
                .withf(move |uid, id| uid == "user-1" && *id == note_id)
                .times(1)
                .return_once(move |_, _| Ok(cloned));

            let result = service.get_note("user-1", note_id).await.unwrap();
            assert_eq!(result.id(), note_id);
        }

        #[tokio::test]
        async fn it_should_return_not_found_for_missing_note() {
            let mut service = create_test_service();

            service
                .repository
                .expect_get()
                .times(1)
                .return_once(|_, _| Err(RepositoryError::NotFound));

            let result = service.get_note("user-1", Uuid::new_v4()).await;
            assert!(matches!(result, Err(NoteServiceError::NotFound)));
        }

        #[tokio::test]
        async fn it_should_create_note_with_generated_id() {
            let mut service = create_test_service();

            service
                .repository
                .expect_create()
                .times(1)
                .returning(|note| Ok(note));

            let result = service
                .create_note("user-1", "Title".to_string(), "Content".to_string())
                .await
                .unwrap();
            assert_eq!(result.user_id(), "user-1");
            assert_eq!(result.title(), "Title");
        }

        #[tokio::test]
        async fn it_should_update_note_title_and_content() {
            let mut service = create_test_service();
            let note = Note::new("user-1".to_string(), "Old".to_string(), "Old".to_string());
            let note_id = note.id();
            let cloned = note.clone();

            service
                .repository
                .expect_get()
                .times(1)
                .return_once(move |_, _| Ok(cloned));

            service
                .repository
                .expect_update()
                .times(1)
                .returning(|_, note| Ok(note));

            let result = service
                .update_note(
                    "user-1",
                    note_id,
                    Some("New".to_string()),
                    Some("New".to_string()),
                )
                .await
                .unwrap();

            assert_eq!(result.title(), "New");
            assert_eq!(result.content(), "New");
        }

        #[tokio::test]
        async fn it_should_delete_note() {
            let mut service = create_test_service();

            service
                .repository
                .expect_delete()
                .times(1)
                .return_once(|_, _| Ok(()));

            assert!(service.delete_note("user-1", Uuid::new_v4()).await.is_ok());
        }

        #[tokio::test]
        async fn it_should_return_forbidden_when_deleting_other_users_note() {
            let mut service = create_test_service();

            service
                .repository
                .expect_delete()
                .times(1)
                .return_once(|_, _| Err(RepositoryError::Forbidden));

            let result = service.delete_note("user-2", Uuid::new_v4()).await;
            assert!(matches!(result, Err(NoteServiceError::Forbidden)));
        }
    }
}
