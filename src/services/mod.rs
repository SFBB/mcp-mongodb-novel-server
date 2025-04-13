pub mod db_service;
pub mod crud_service;

pub use db_service::{DatabaseService, MongoDBService};
pub use crud_service::{
    CrudService, 
    NovelCrudService, 
    ChapterCrudService, 
    CharacterCrudService, 
    QACrudService
};