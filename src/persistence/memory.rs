//! In-memory repository implementation.
//!
//! This is a simple implementation for educational purposes.
//! In a real application, you would implement `NoteRepository` for a database client.

use std::collections::HashMap;
use std::sync::RwLock;

use error_stack::{Report, ResultExt};
use uuid::Uuid;

use super::{NoteRepository, RepositoryError};
use crate::domain::note::Note;

/// In-memory note storage using a `RwLock<HashMap>`.
///
/// This implementation demonstrates the repository pattern without external dependencies.
/// Data is lost when the server stops — this is intentional for educational purposes.
#[derive(Debug)]
pub struct InMemoryNoteRepository {
    notes: RwLock<HashMap<Uuid, Note>>,
}

impl InMemoryNoteRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            notes: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryNoteRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryNoteRepository {
    fn clone(&self) -> Self {
        let notes = self.notes.read().expect("RwLock poisoned").clone();
        Self {
            notes: RwLock::new(notes),
        }
    }
}

impl NoteRepository for InMemoryNoteRepository {
    async fn list(&self, user_id: &str) -> Result<Vec<Note>, Report<RepositoryError>> {
        let notes = self
            .notes
            .read()
            .map_err(|_| Report::new(RepositoryError::StorageError))
            .attach_printable("Failed to acquire read lock for listing notes")?;

        let user_notes: Vec<Note> = notes
            .values()
            .filter(|note| note.user_id() == user_id)
            .cloned()
            .collect();

        Ok(user_notes)
    }

    async fn get(&self, user_id: &str, note_id: Uuid) -> Result<Note, Report<RepositoryError>> {
        let notes = self
            .notes
            .read()
            .map_err(|_| Report::new(RepositoryError::StorageError))
            .attach_printable("Failed to acquire read lock for getting note")?;

        let note = notes
            .get(&note_id)
            .ok_or_else(|| Report::new(RepositoryError::NotFound { note_id }))?;

        if note.user_id() != user_id {
            return Err(Report::new(RepositoryError::Forbidden { note_id }));
        }

        Ok(note.clone())
    }

    async fn create(&self, note: Note) -> Result<Note, Report<RepositoryError>> {
        let mut notes = self
            .notes
            .write()
            .map_err(|_| Report::new(RepositoryError::StorageError))
            .attach_printable("Failed to acquire write lock for creating note")?;

        notes.insert(note.id(), note.clone());

        Ok(note)
    }

    async fn update(&self, user_id: &str, note: Note) -> Result<Note, Report<RepositoryError>> {
        let mut notes = self
            .notes
            .write()
            .map_err(|_| Report::new(RepositoryError::StorageError))
            .attach_printable("Failed to acquire write lock for updating note")?;

        let note_id = note.id();
        let existing = notes
            .get(&note_id)
            .ok_or_else(|| Report::new(RepositoryError::NotFound { note_id }))?;

        if existing.user_id() != user_id {
            return Err(Report::new(RepositoryError::Forbidden { note_id }));
        }

        notes.insert(note_id, note.clone());

        Ok(note)
    }

    async fn delete(&self, user_id: &str, note_id: Uuid) -> Result<(), Report<RepositoryError>> {
        let mut notes = self
            .notes
            .write()
            .map_err(|_| Report::new(RepositoryError::StorageError))
            .attach_printable("Failed to acquire write lock for deleting note")?;

        let existing = notes
            .get(&note_id)
            .ok_or_else(|| Report::new(RepositoryError::NotFound { note_id }))?;

        if existing.user_id() != user_id {
            return Err(Report::new(RepositoryError::Forbidden { note_id }));
        }

        notes.remove(&note_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_note(user_id: &str, title: &str) -> Note {
        Note::new(
            user_id.to_string(),
            title.to_string(),
            "Content".to_string(),
        )
    }

    #[tokio::test]
    async fn it_should_return_empty_list_for_new_repository() {
        let repo = InMemoryNoteRepository::new();
        let notes = repo.list("user-1").await.unwrap();
        assert!(notes.is_empty());
    }

    #[tokio::test]
    async fn it_should_only_list_notes_for_specified_user() {
        let repo = InMemoryNoteRepository::new();
        let note1 = create_test_note("user-1", "Note 1");
        let note2 = create_test_note("user-2", "Note 2");
        let note3 = create_test_note("user-1", "Note 3");

        repo.create(note1.clone()).await.unwrap();
        repo.create(note2).await.unwrap();
        repo.create(note3.clone()).await.unwrap();

        let user1_notes = repo.list("user-1").await.unwrap();

        assert_eq!(user1_notes.len(), 2);
        assert!(user1_notes.iter().any(|n| n.id() == note1.id()));
        assert!(user1_notes.iter().any(|n| n.id() == note3.id()));
    }

    #[tokio::test]
    async fn it_should_create_and_retrieve_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();

        let created = repo.create(note).await.unwrap();
        assert_eq!(created.id(), note_id);

        let retrieved = repo.get("user-1", note_id).await.unwrap();
        assert_eq!(retrieved.title(), "My Note");
    }

    #[tokio::test]
    async fn it_should_return_not_found_for_missing_note() {
        let repo = InMemoryNoteRepository::new();
        let result = repo.get("user-1", Uuid::new_v4()).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_accessing_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let result = repo.get("user-2", note_id).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::Forbidden { .. }
        ));
    }

    #[tokio::test]
    async fn it_should_update_existing_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "Original");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let mut retrieved = repo.get("user-1", note_id).await.unwrap();
        retrieved.set_title("Updated".to_string());
        repo.update("user-1", retrieved).await.unwrap();

        let updated = repo.get("user-1", note_id).await.unwrap();
        assert_eq!(updated.title(), "Updated");
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_updating_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let retrieved = repo.get("user-1", note_id).await.unwrap();
        let result = repo.update("user-2", retrieved).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::Forbidden { .. }
        ));
    }

    #[tokio::test]
    async fn it_should_delete_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        repo.delete("user-1", note_id).await.unwrap();

        let result = repo.get("user-1", note_id).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn it_should_return_forbidden_when_deleting_other_users_note() {
        let repo = InMemoryNoteRepository::new();
        let note = create_test_note("user-1", "My Note");
        let note_id = note.id();
        repo.create(note).await.unwrap();

        let result = repo.delete("user-2", note_id).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::Forbidden { .. }
        ));
    }

    #[tokio::test]
    async fn it_should_return_not_found_when_deleting_missing_note() {
        let repo = InMemoryNoteRepository::new();
        let result = repo.delete("user-1", Uuid::new_v4()).await;
        assert!(matches!(
            result.unwrap_err().current_context(),
            RepositoryError::NotFound { .. }
        ));
    }
}
