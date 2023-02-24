use super::product::Product;

pub struct ProductEvent {
    pub product_id: String,
    pub product: Option<Product>,
}