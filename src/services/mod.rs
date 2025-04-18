pub mod crud_service;
pub mod db_service;

pub use db_service::{DatabaseService};
pub use crud_service::{
    NovelCrudService, ChapterCrudService, CharacterCrudService, QACrudService
};