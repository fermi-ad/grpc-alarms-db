pub mod postgres;

pub trait DataRow {
    fn get_str_value(&self, column_name: &str) -> String;
    fn get_i32_value(&self, column_name: &str) -> i32;
}

#[tonic::async_trait]
pub trait DataStore<T: DataRow> {
    async fn execute_query(&self, query: &str) -> Vec<T>;   
}