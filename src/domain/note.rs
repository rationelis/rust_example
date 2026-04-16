//! Note entity and service.
//!
//! This module contains the core business logic for notes. It has no knowledge
//! of HTTP, databases, or any I/O — it depends only on the `NoteRepository` trait.

use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use thiserror::Error;
use uuid::Uuid;

use crate::persistence::{NoteRepository, RepositoryError};

/// A note entity representing a user's note.
///
/// Notes are immutable except through the `set_*` methods, which also update
/// the `updated_at` timestamp.
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
    /// Creates a new note with a generated ID and current timestamps.
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

    /// Reconstructs a note from its parts (e.g., from database).
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

// ============================================================================
// Domain Errors
// ============================================================================

/// Errors that can occur when listing notes.
///
/// Following the error-stack pattern: domain errors + single `Unexpected` variant.
#[derive(Debug, Error)]
pub enum ListNotesError {
    /// An unexpected infrastructure error occurred.
    #[error("An unexpected error occurred")]
    Unexpected,
}

/// Errors that can occur when getting a note.
#[derive(Debug, Error)]
pub enum GetNoteError {
    /// The note was not found.
    #[error("Note with id {note_id} not found")]
    NotFound { note_id: Uuid },

    /// The user is not allowed to access this note.
    #[error("Access denied to note {note_id}")]
    Forbidden { note_id: Uuid },

    /// An unexpected infrastructure error occurred.
    #[error("An unexpected error occurred")]
    Unexpected,
}

/// Errors that can occur when creating a note.
#[derive(Debug, Error)]
pub enum CreateNoteError {
    /// An unexpected infrastructure error occurred.
    #[error("An unexpected error occurred")]
    Unexpected,
}

/// Errors that can occur when updating a note.
#[derive(Debug, Error)]
pub enum UpdateNoteError {
    /// The note was not found.
    #[error("Note with id {note_id} not found")]
    NotFound { note_id: Uuid },

    /// The user is not allowed to update this note.
    #[error("Access denied to note {note_id}")]
    Forbidden { note_id: Uuid },

    /// An unexpected infrastructure error occurred.
    #[error("An unexpected error occurred")]
    Unexpected,
}

/// Errors that can occur when deleting a note.
#[derive(Debug, Error)]
pub enum DeleteNoteError {
    /// The note was not found.
    #[error("Note with id {note_id} not found")]
    NotFound { note_id: Uuid },

    /// The user is not allowed to delete this note.
    #[error("Access denied to note {note_id}")]
    Forbidden { note_id: Uuid },

    /// An unexpected infrastructure error occurred.
    #[error("An unexpected error occurred")]
    Unexpected,
}

// ============================================================================
// Service
// ============================================================================

/// Generic service that accepts any `NoteRepository` implementation.
///
/// This demonstrates dependency inversion: the service depends on an abstract
/// trait, not a concrete implementation.
#[derive(Debug, Clone)]
pub struct NoteService<R: NoteRepository> {
    repository: R,
}

impl<R: NoteRepository> NoteService<R> {
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Lists all notes for a user.
    pub async fn list_notes(&self, user_id: &str) -> Result<Vec<Note>, Report<ListNotesError>> {
        tracing::debug!(user_id = %user_id, "Listing notes");

        let notes = self
            .repository
            .list(user_id)
            .await
            .change_context(ListNotesError::Unexpected)
            .attach_printable(format!("Failed to list notes for user {user_id}"))?;

        tracing::debug!(user_id = %user_id, count = notes.len(), "Found notes");
        Ok(notes)
    }

    /// Gets a single note by ID.
    pub async fn get_note(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> Result<Note, Report<GetNoteError>> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Getting note");

        self.repository
            .get(user_id, note_id)
            .await
            .map_err(|report| map_repository_error_to_get(report, note_id))
    }

    /// Creates a new note.
    pub async fn create_note(
        &self,
        user_id: &str,
        title: String,
        content: String,
    ) -> Result<Note, Report<CreateNoteError>> {
        tracing::debug!(user_id = %user_id, title = %title, "Creating note");

        let note = Note::new(user_id.to_string(), title, content);

        let created = self
            .repository
            .create(note)
            .await
            .change_context(CreateNoteError::Unexpected)
            .attach_printable(format!("Failed to create note for user {user_id}"))?;

        tracing::info!(user_id = %user_id, note_id = %created.id(), "Note created");
        Ok(created)
    }

    /// Updates an existing note.
    pub async fn update_note(
        &self,
        user_id: &str,
        note_id: Uuid,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Note, Report<UpdateNoteError>> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Updating note");

        // First, get the existing note
        let mut note = self
            .repository
            .get(user_id, note_id)
            .await
            .map_err(|report| map_repository_error_to_update(report, note_id))?;

        // Apply updates
        if let Some(t) = title {
            note.set_title(t);
        }
        if let Some(c) = content {
            note.set_content(c);
        }

        // Save the updated note
        let updated = self
            .repository
            .update(user_id, note)
            .await
            .map_err(|report| map_repository_error_to_update(report, note_id))?;

        tracing::info!(user_id = %user_id, note_id = %note_id, "Note updated");
        Ok(updated)
    }

    /// Deletes a note.
    pub async fn delete_note(
        &self,
        user_id: &str,
        note_id: Uuid,
    ) -> Result<(), Report<DeleteNoteError>> {
        tracing::debug!(user_id = %user_id, note_id = %note_id, "Deleting note");

        self.repository
            .delete(user_id, note_id)
            .await
            .map_err(|report| map_repository_error_to_delete(report, note_id))?;

        tracing::info!(user_id = %user_id, note_id = %note_id, "Note deleted");
        Ok(())
    }
}

// ============================================================================
// Error Mapping Helpers
// ============================================================================

/// Maps repository errors to GetNoteError.
///
/// Domain errors (NotFound, Forbidden) are preserved; infrastructure errors
/// become `Unexpected`.
fn map_repository_error_to_get(
    report: Report<RepositoryError>,
    note_id: Uuid,
) -> Report<GetNoteError> {
    match report.current_context() {
        RepositoryError::NotFound { .. } => {
            report.change_context(GetNoteError::NotFound { note_id })
        }
        RepositoryError::Forbidden { .. } => {
            report.change_context(GetNoteError::Forbidden { note_id })
        }
        RepositoryError::StorageError => report
            .change_context(GetNoteError::Unexpected)
            .attach_printable(format!("Failed to get note {note_id}")),
    }
}

/// Maps repository errors to UpdateNoteError.
fn map_repository_error_to_update(
    report: Report<RepositoryError>,
    note_id: Uuid,
) -> Report<UpdateNoteError> {
    match report.current_context() {
        RepositoryError::NotFound { .. } => {
            report.change_context(UpdateNoteError::NotFound { note_id })
        }
        RepositoryError::Forbidden { .. } => {
            report.change_context(UpdateNoteError::Forbidden { note_id })
        }
        RepositoryError::StorageError => report
            .change_context(UpdateNoteError::Unexpected)
            .attach_printable(format!("Failed to update note {note_id}")),
    }
}

/// Maps repository errors to DeleteNoteError.
fn map_repository_error_to_delete(
    report: Report<RepositoryError>,
    note_id: Uuid,
) -> Report<DeleteNoteError> {
    match report.current_context() {
        RepositoryError::NotFound { .. } => {
            report.change_context(DeleteNoteError::NotFound { note_id })
        }
        RepositoryError::Forbidden { .. } => {
            report.change_context(DeleteNoteError::Forbidden { note_id })
        }
        RepositoryError::StorageError => report
            .change_context(DeleteNoteError::Unexpected)
            .attach_printable(format!("Failed to delete note {note_id}")),
    }
}

// ============================================================================
// Tests
// ============================================================================

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
            let note_id = Uuid::new_v4();

            service
                .repository
                .expect_get()
                .times(1)
                .return_once(move |_, _| Err(Report::new(RepositoryError::NotFound { note_id })));

            let result = service.get_note("user-1", note_id).await;
            assert!(matches!(
                result.unwrap_err().current_context(),
                GetNoteError::NotFound { .. }
            ));
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
            let note_id = Uuid::new_v4();

            service
                .repository
                .expect_delete()
                .times(1)
                .return_once(move |_, _| Err(Report::new(RepositoryError::Forbidden { note_id })));

            let result = service.delete_note("user-2", note_id).await;
            assert!(matches!(
                result.unwrap_err().current_context(),
                DeleteNoteError::Forbidden { .. }
            ));
        }
    }
}
