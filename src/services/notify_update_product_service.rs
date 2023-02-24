
use async_trait::async_trait;

#[async_trait]
pub trait NotifyUpdateProductService {
    async fn notify_product_update(&self, product_id: &String);
    async fn notify_product_delete(&self, product_id: &String);
}

#[derive(strum_macros::Display)]
pub enum CommandType {
    UPDATE,
    DELETE,
}
