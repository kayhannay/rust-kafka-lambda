use std::collections::HashSet;
use chrono::{DateTime, FixedOffset};
use serde::{Serialize,Deserialize};

#[derive(Default)]
#[derive(Serialize)]
#[derive(Deserialize)]
#[derive(PartialEq)]
#[derive(Debug)]
pub struct ConvertedProduct {
    #[serde(rename = "productId")]
    pub product_id: String,
    pub title: String,
    pub price: f64,
    #[serde(rename = "productGroups")]
    pub product_groups: Option<HashSet<String>>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<DateTime<FixedOffset>>,
    pub dimensions: Option<Dimensions>,
}

#[derive(Default)]
#[derive(Serialize)]
#[derive(Deserialize)]
#[derive(PartialEq)]
#[derive(Debug)]
pub struct Dimensions {
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}
