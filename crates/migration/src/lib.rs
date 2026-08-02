use sea_orm_migration::prelude::*;

mod m20260802_000001_create_wordbook;
mod m20260802_000002_create_word;
mod m20260802_000003_add_word_indexes;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260802_000001_create_wordbook::Migration),
            Box::new(m20260802_000002_create_word::Migration),
            Box::new(m20260802_000003_add_word_indexes::Migration),
        ]
    }
}
