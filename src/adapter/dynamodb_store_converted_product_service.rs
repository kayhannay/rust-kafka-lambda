use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::model::AttributeValue;
use async_trait::async_trait;
use crate::domain::converted_product::{ConvertedProduct};
use crate::services::store_converted_product_service::StoreConvertedProductService;

pub struct DynamoDbStoreConvertedProductService {
    pub dynamo_db_client: Client,
    pub table_name: String,
}

#[async_trait]
impl StoreConvertedProductService for DynamoDbStoreConvertedProductService {
    async fn store(&self, converted_product: &ConvertedProduct) -> bool {
        if self.has_changes(converted_product).await {
            self.dynamo_db_client.put_item()
                .table_name(&self.table_name)
                .item("id", AttributeValue::S(converted_product.product_id.to_string()))
                .item("data", AttributeValue::S(serde_json::to_string(&converted_product).unwrap()))
                .send()
                .await
                .expect(&*format!("Could not store product {} in DynamoDB!", &converted_product.product_id));
            true
        } else {
            false
        }
    }

    async fn delete(&self, product_id: &String) -> bool {
        if self.get_by_id(product_id).await.is_some() {
            self.dynamo_db_client.delete_item()
                .table_name(&self.table_name)
                .key("id", AttributeValue::S(product_id.clone()))
                .send()
                .await
                .expect(&*format!("Could not delete product {} from DynamoDB!", &product_id));
            true
        } else {
            false
        }
    }
}

impl DynamoDbStoreConvertedProductService {

    async fn has_changes(&self, converted_product: &ConvertedProduct) -> bool {
        let attribute_value = self.get_by_id(&converted_product.product_id).await;
        if attribute_value.is_some() {
            let data = attribute_value.unwrap();
            let strin = data.as_s().unwrap();
            let stored_product: ConvertedProduct = serde_json::from_str(strin).expect("Could not parse product");
            &stored_product != converted_product
        } else {
            true
        }
    }

    async fn get_by_id(&self, product_id: &String) -> Option<AttributeValue> {
        match self.dynamo_db_client.get_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(product_id.clone()))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.item().is_some() && resp.item().unwrap().contains_key("data") {
                    Some(resp.item().unwrap().get("data").unwrap().clone())
                } else {
                    None
                }
            }
            Err(_) => {
                None
            }
        }
    }

}