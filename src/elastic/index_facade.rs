use super::error::ElasticError;
use super::index_definition::IndexDefinition;
use super::index_repository::IndexRepository;
use super::product_index::ProductIndex;
use indicatif::HumanDuration;
use std::cmp::{max, min};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

pub struct IndexFacade<'a> {
    index_repository: &'a IndexRepository,
}

impl<'a> IndexFacade<'a> {
    pub fn new(index_repository: &'a IndexRepository) -> Self {
        IndexFacade { index_repository }
    }

    pub fn create(&self, index_definition: &IndexDefinition) {}

    pub async fn export(&self, product_index: &ProductIndex, index_definition: &IndexDefinition) {
        println!(
            "Exporting data of '{}' on domain '{}'",
            index_definition.index_name, index_definition.domain_id
        );

        self.create_index_when_no_alias_found(index_definition);

        let bar = indicatif::ProgressBar::new(650000);
        bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({percent}%) ETA: {eta} Postgres: {msg}",
                )
                .unwrap()
                .progress_chars("=> "),
        );
        let mut last_processed_id = 0;
        let batch_size = 100;

        let mut postgres_duration = Duration::new(0, 0);
        let n_max = 3;

        let single_thread = true;

        loop {
            if single_thread {
                // let now = Instant::now();
                let current_batch_data = product_index
                    .get_export_data_for_batch(
                        index_definition.domain_id,
                        last_processed_id,
                        batch_size,
                    )
                    .await;
                // postgres_duration += now.elapsed();
                // bar.set_message(format!("{}", HumanDuration(postgres_duration)));

                self.index_repository
                    .bulk_update(index_definition, &current_batch_data)
                    .await;

                if let Some(last_key) = current_batch_data.keys().last() {
                    last_processed_id = *last_key as u32;
                }

                bar.inc(current_batch_data.len() as u64);

                if (current_batch_data.len() as u32) < batch_size {
                    break;
                }
            } else {
                let mut tasks = Vec::with_capacity(n_max);
                for n in (0..n_max) {
                    let product_index_clone = product_index.clone();
                    let index_definition_clone = index_definition.clone();
                    let index_repository_clone = self.index_repository.clone();
                    tasks.push(tokio::spawn(get_batch_and_bulk_update(
                        index_repository_clone,
                        product_index_clone,
                        index_definition_clone,
                        last_processed_id + (n as u32 * batch_size),
                        batch_size,
                    )));
                }

                let mut outputs = Vec::with_capacity(n_max);
                for task in tasks {
                    outputs.push(task.await.unwrap());
                }

                last_processed_id = outputs.iter().map(|i| i.1 as u32).max().unwrap();

                bar.inc(outputs.iter().map(|i| i.0 as u64).sum());

                if (outputs.iter().map(|i| i.0 as u32).min().unwrap()) < batch_size {
                    break;
                }
            }
        }

        bar.finish();
        return;
        todo!("is_index_up_to_date > migrate")
    }

    pub async fn migrate(&self, index_definition: &IndexDefinition) {
        let existing_index_name = self.resolve_existing_index_name(index_definition).await;
        if let Err(ElasticError::NoAlias) = existing_index_name {
            println!(
                "No index for alias \"{}\" was not found on domain \"{}\"",
                index_definition.index_name, index_definition.domain_id
            );
            self.create(index_definition);
            return;
        }

        let existing_index_name = existing_index_name.unwrap();
    }

    async fn create_index_when_no_alias_found(&self, index_definition: &IndexDefinition) {
        if let Err(ElasticError::NoAlias) = self
            .index_repository
            .find_current_index_name_for_alias(&index_definition.get_index_alias())
            .await
        {
            panic!(
                "Index '{}' does not exist on domain '{}'",
                index_definition.index_name, index_definition.domain_id
            );
        } else {
            todo!("self.create")
        }
    }

    pub async fn resolve_existing_index_name(
        &self,
        index_definition: &IndexDefinition,
    ) -> Result<String, ElasticError> {
        self.index_repository
            .find_current_index_name_for_alias(&index_definition.get_index_alias())
            .await
    }
}

async fn get_batch_and_bulk_update(
    index_repository: IndexRepository,
    product_index: ProductIndex,
    index_definition: IndexDefinition,
    last_processed_id: u32,
    batch_size: u32,
) -> (usize, i32) {
    let current_batch_data = product_index
        .get_export_data_for_batch(index_definition.domain_id, last_processed_id, batch_size)
        .await;

    index_repository
        .bulk_update(&index_definition, &current_batch_data)
        .await;

    return (
        current_batch_data.len(),
        *current_batch_data.keys().last().unwrap(),
    );
}
