#[macro_use]
extern crate diesel;

pub mod models;
pub mod queries;
pub mod schema;

pub use diesel::insert_into;
pub use diesel::prelude::*;
pub use diesel::result::Error as DieselError;
