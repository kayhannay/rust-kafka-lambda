use crate::services::notify_update_product_service::{CommandType, NotifyUpdateProductService};
use async_trait::async_trait;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use std::time::Duration;

pub struct KafkaNotifyUpdateProductService {
    pub producer: FutureProducer,
    pub topic_name: String,
}

#[async_trait]
impl NotifyUpdateProductService for KafkaNotifyUpdateProductService {
    async fn notify_product_update(&self, product_id: &str) {
        self.producer
            .send(
                FutureRecord::to(&self.topic_name)
                    .key(&product_id.to_string())
                    .payload(&CommandType::UPDATE.to_string()),
                Timeout::After(Duration::from_millis(0)),
            )
            .await
            .expect("Could not send update command to Kafka");
    }

    async fn notify_product_delete(&self, product_id: &str) {
        self.producer
            .send(
                FutureRecord::to(&self.topic_name)
                    .key(&product_id.to_string())
                    .payload(&CommandType::DELETE.to_string()),
                Timeout::After(Duration::from_millis(0)),
            )
            .await
            .expect("Could not send delete command to Kafka");
    }
}
