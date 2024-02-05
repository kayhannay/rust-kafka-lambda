use chrono::DateTime;
use std::collections::HashSet;

use crate::domain::converted_product::{ConvertedProduct, Dimensions};
use crate::domain::product::Product;

pub fn convert_product(product: &Product) -> ConvertedProduct {
    return ConvertedProduct {
        product_id: product.id.clone(),
        title: product.name.clone(),
        price: product.price,
        last_modified: product
            .last_modified
            .as_ref()
            .and_then(|last_modified| DateTime::parse_from_rfc3339(last_modified).ok()),
        product_groups: product
            .tags
            .as_ref()
            .map(|tag| HashSet::from_iter(tag.iter().cloned())),
        dimensions: product.dimensions.as_ref().map(|dimensions| Dimensions {
            length: Some(dimensions.length),
            height: Some(dimensions.height),
            width: Some(dimensions.width),
        }),
    };
}
