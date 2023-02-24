use slog::{Logger, info};

use crate::adapter::dynamodb_store_converted_product_service::DynamoDbStoreConvertedProductService;
use crate::adapter::kafka_notify_update_product_service::KafkaNotifyUpdateProductService;
use crate::business;
use crate::services::notify_update_product_service::NotifyUpdateProductService;
use crate::services::store_converted_product_service::StoreConvertedProductService;
use crate::domain::product_event::ProductEvent;

pub struct SaveConvertedProductUseCase {
    pub log: Logger,
    pub store_converted_product_service: DynamoDbStoreConvertedProductService,
    pub notify_update_product_service: KafkaNotifyUpdateProductService
}

impl SaveConvertedProductUseCase {

    pub async fn execute(&self, product_events: Vec<ProductEvent>) {
        for product_event in product_events {
            self.handle_product(&product_event).await;
        }
    }

    async fn handle_product(&self, product_event: &ProductEvent) {
        let product_id = product_event.product_id.clone();
        info!(self.log, "Import product {} ...", product_id);
        let product_option = product_event.product.as_ref();
        if product_option.is_none() {
            let deleted = self.store_converted_product_service.delete(&product_id).await;
            if deleted {
                self.notify_update_product_service.notify_product_delete(&product_id).await;
                info!(self.log, "Product {} deleted successfully", product_id);
            }
        } else {
            let product = product_option.unwrap();

            // convert
            let converted_product = business::convert_use_case::convert_product(product);

            // store in DynamoDB
            info!(self.log, "Store product {} ...", product_id);
            let stored = self.store_converted_product_service.store(&converted_product).await;
            if stored {
                info!(self.log, "Send notification for product {} ...", product_id);
                self.notify_update_product_service.notify_product_update(&converted_product.product_id.to_string()).await;
                info!(self.log, "Product {} imported successfully", product_id);
            }
        }
    }

}
