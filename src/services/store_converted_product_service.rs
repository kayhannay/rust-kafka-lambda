use crate::domain::converted_product::ConvertedProduct;
use async_trait::async_trait;

#[async_trait]
pub trait StoreConvertedProductService {
    async fn store(&self, converted_product: &ConvertedProduct) -> bool;
    async fn delete(&self, product_id: &str) -> bool;
}
