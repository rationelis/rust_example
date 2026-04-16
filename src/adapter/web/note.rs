//! Note HTTP handlers.
//!
//! This module handles HTTP ↔ Domain translation. It uses `gen_error_response!`
//! to define web-layer errors with HTTP status codes, and maps domain errors
//! to appropriate HTTP responses.

use chrono::{DateTime, Utc};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, ApiResponse, Object, OpenApi};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    adapter::web::gen_error_response,
    auth::AuthenticatedUser,
    domain::note::{
        CreateNoteError as DomainCreateNoteError, DeleteNoteError as DomainDeleteNoteError,
        GetNoteError as DomainGetNoteError, ListNotesError as DomainListNotesError, Note,
        UpdateNoteError as DomainUpdateNoteError,
    },
    NoteService,
};

#[derive(Debug, Clone, Copy)]
pub struct NoteApi;

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize, Object)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Object)]
pub struct UpdateNoteRequest {
    #[oai(skip_serializing_if_is_none)]
    pub title: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Object)]
pub struct NoteResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Note> for NoteResponse {
    fn from(note: Note) -> Self {
        Self {
            id: note.id(),
            title: note.title().to_string(),
            content: note.content().to_string(),
            created_at: note.created_at(),
            updated_at: note.updated_at(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Object)]
pub struct NotesListResponse {
    pub notes: Vec<NoteResponse>,
    pub count: usize,
}

impl From<Vec<Note>> for NotesListResponse {
    fn from(notes: Vec<Note>) -> Self {
        let count = notes.len();
        Self {
            notes: notes.into_iter().map(NoteResponse::from).collect(),
            count,
        }
    }
}

// ============================================================================
// API Responses (Success)
// ============================================================================

#[derive(Debug, ApiResponse)]
pub enum ListNotesResponse {
    #[oai(status = 200)]
    Ok(Json<NotesListResponse>),
}

#[derive(Debug, ApiResponse)]
pub enum GetNoteResponse {
    #[oai(status = 200)]
    Ok(Json<NoteResponse>),
}

#[derive(Debug, ApiResponse)]
pub enum CreateNoteResponse {
    #[oai(status = 201)]
    Created(Json<NoteResponse>),
}

#[derive(Debug, ApiResponse)]
pub enum UpdateNoteResponse {
    #[oai(status = 200)]
    Ok(Json<NoteResponse>),
}

#[derive(Debug, ApiResponse)]
pub enum DeleteNoteResponse {
    #[oai(status = 204)]
    NoContent,
}

// ============================================================================
// Error Responses (Web Layer)
//
// These are separate from domain errors. The handler maps domain errors
// to these web-layer errors which have HTTP status codes.
// ============================================================================

gen_error_response! {
    ListNotesError {
        InternalServerError -> (500, "Failed to list notes"),
    }
}

gen_error_response! {
    GetNoteError {
        Forbidden -> (403, "You don't have access to this note"),
        NotFound -> (404, "Note not found"),
        InternalServerError -> (500, "Failed to retrieve note"),
    }
}

gen_error_response! {
    CreateNoteError {
        BadRequest -> (400, "Invalid note data"),
        InternalServerError -> (500, "Failed to create note"),
    }
}

gen_error_response! {
    UpdateNoteError {
        BadRequest -> (400, "Invalid note data"),
        Forbidden -> (403, "You don't have access to this note"),
        NotFound -> (404, "Note not found"),
        InternalServerError -> (500, "Failed to update note"),
    }
}

gen_error_response! {
    DeleteNoteError {
        Forbidden -> (403, "You don't have access to this note"),
        NotFound -> (404, "Note not found"),
        InternalServerError -> (500, "Failed to delete note"),
    }
}

// ============================================================================
// API Implementation
// ============================================================================

#[OpenApi(prefix_path = "/notes", tag = "super::NotesApiTags::Notes")]
impl NoteApi {
    /// List all notes for the authenticated user.
    #[oai(path = "/", method = "get", operation_id = "list_notes")]
    pub async fn list_notes(
        &self,
        user: AuthenticatedUser,
        Data(service): Data<&NoteService>,
    ) -> Result<ListNotesResponse, ListNotesError> {
        tracing::info!(user_id = %user.user_id(), "Listing notes");

        match service.list_notes(user.user_id()).await {
            Ok(notes) => Ok(ListNotesResponse::Ok(Json(notes.into()))),
            Err(error) => match error.current_context() {
                DomainListNotesError::Unexpected => {
                    tracing::error!(?error, "Unexpected error listing notes");
                    Err(ListNotesError::InternalServerError)
                }
            },
        }
    }

    /// Create a new note.
    #[oai(path = "/", method = "post", operation_id = "create_note")]
    pub async fn create_note(
        &self,
        user: AuthenticatedUser,
        Data(service): Data<&NoteService>,
        Json(request): Json<CreateNoteRequest>,
    ) -> Result<CreateNoteResponse, CreateNoteError> {
        tracing::info!(user_id = %user.user_id(), title = %request.title, "Creating note");

        // Validate input
        if request.title.trim().is_empty() {
            return Err(CreateNoteError::BadRequest);
        }

        match service
            .create_note(user.user_id(), request.title, request.content)
            .await
        {
            Ok(note) => Ok(CreateNoteResponse::Created(Json(note.into()))),
            Err(error) => match error.current_context() {
                DomainCreateNoteError::Unexpected => {
                    tracing::error!(?error, "Unexpected error creating note");
                    Err(CreateNoteError::InternalServerError)
                }
            },
        }
    }

    /// Get a specific note by ID.
    #[oai(path = "/:note_id", method = "get", operation_id = "get_note")]
    pub async fn get_note(
        &self,
        user: AuthenticatedUser,
        Data(service): Data<&NoteService>,
        Path(note_id): Path<Uuid>,
    ) -> Result<GetNoteResponse, GetNoteError> {
        tracing::info!(user_id = %user.user_id(), note_id = %note_id, "Getting note");

        match service.get_note(user.user_id(), note_id).await {
            Ok(note) => Ok(GetNoteResponse::Ok(Json(note.into()))),
            Err(error) => match error.current_context() {
                DomainGetNoteError::NotFound { .. } => Err(GetNoteError::NotFound),
                DomainGetNoteError::Forbidden { .. } => Err(GetNoteError::Forbidden),
                DomainGetNoteError::Unexpected => {
                    tracing::error!(?error, "Unexpected error getting note");
                    Err(GetNoteError::InternalServerError)
                }
            },
        }
    }

    /// Update an existing note.
    #[oai(path = "/:note_id", method = "put", operation_id = "update_note")]
    pub async fn update_note(
        &self,
        user: AuthenticatedUser,
        Data(service): Data<&NoteService>,
        Path(note_id): Path<Uuid>,
        Json(request): Json<UpdateNoteRequest>,
    ) -> Result<UpdateNoteResponse, UpdateNoteError> {
        tracing::info!(user_id = %user.user_id(), note_id = %note_id, "Updating note");

        // Validate input
        if let Some(ref title) = request.title {
            if title.trim().is_empty() {
                return Err(UpdateNoteError::BadRequest);
            }
        }

        match service
            .update_note(user.user_id(), note_id, request.title, request.content)
            .await
        {
            Ok(note) => Ok(UpdateNoteResponse::Ok(Json(note.into()))),
            Err(error) => match error.current_context() {
                DomainUpdateNoteError::NotFound { .. } => Err(UpdateNoteError::NotFound),
                DomainUpdateNoteError::Forbidden { .. } => Err(UpdateNoteError::Forbidden),
                DomainUpdateNoteError::Unexpected => {
                    tracing::error!(?error, "Unexpected error updating note");
                    Err(UpdateNoteError::InternalServerError)
                }
            },
        }
    }

    /// Delete a note.
    #[oai(path = "/:note_id", method = "delete", operation_id = "delete_note")]
    pub async fn delete_note(
        &self,
        user: AuthenticatedUser,
        Data(service): Data<&NoteService>,
        Path(note_id): Path<Uuid>,
    ) -> Result<DeleteNoteResponse, DeleteNoteError> {
        tracing::info!(user_id = %user.user_id(), note_id = %note_id, "Deleting note");

        match service.delete_note(user.user_id(), note_id).await {
            Ok(()) => Ok(DeleteNoteResponse::NoContent),
            Err(error) => match error.current_context() {
                DomainDeleteNoteError::NotFound { .. } => Err(DeleteNoteError::NotFound),
                DomainDeleteNoteError::Forbidden { .. } => Err(DeleteNoteError::Forbidden),
                DomainDeleteNoteError::Unexpected => {
                    tracing::error!(?error, "Unexpected error deleting note");
                    Err(DeleteNoteError::InternalServerError)
                }
            },
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    mod dto_conversions {
        use super::super::*;

        #[test]
        fn it_should_convert_note_to_response() {
            let note = Note::new(
                "user-1".to_string(),
                "Title".to_string(),
                "Content".to_string(),
            );
            let response: NoteResponse = note.clone().into();
            assert_eq!(response.id, note.id());
            assert_eq!(response.title, "Title");
        }

        #[test]
        fn it_should_convert_notes_vec_to_list_response() {
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
            let response: NotesListResponse = notes.into();
            assert_eq!(response.count, 2);
            assert_eq!(response.notes.len(), 2);
        }
    }
}
