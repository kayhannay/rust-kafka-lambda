use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use std::time::Duration;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::{Client, Endpoint, Credentials};
use aws_sdk_dynamodb::model::{AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput, ScalarAttributeType};
use chrono::DateTime;
use futures::StreamExt;
use http::Uri;
use lambda_runtime::LambdaEvent;
use rdkafka::{ClientConfig, Message};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::FutureProducer;
use slog::{Drain, o, Logger};
use testcontainers::clients;
use testcontainers::images::kafka;
use lib_base64::Base64;
use rust_kafka_lambda::adapter::dynamodb_store_converted_product_service::DynamoDbStoreConvertedProductService;
use rust_kafka_lambda::adapter::kafka_notify_update_product_service::KafkaNotifyUpdateProductService;
use rust_kafka_lambda::business::save_converted_product_use_case::SaveConvertedProductUseCase;
use rust_kafka_lambda::domain::kafka_event::{KafkaEvent, KafkaRecord, MillisecondTimestamp};
use rust_kafka_lambda::domain::product::Product;
use rust_kafka_lambda::handler::lambda_kafka_event_handler::LambdaKafkaEventHandler;
use rust_kafka_lambda;

mod localstack;

#[tokio::test]
async fn happy_path() {
    let logger = initialize_logger();
    let docker = clients::Cli::default();
    let localstack = docker.run(localstack::Localstack::default());
    let localstack_port = localstack.get_host_port_ipv4(4566);
    let kafka_node = docker.run(kafka::Kafka::default());
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT)
    );


    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;
    let check_client = client.clone();

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        log: Logger::new(&logger, o!("logger" => "LambdaKafkaEventHandler")),
        save_converted_product_use_case: SaveConvertedProductUseCase {
            log: Logger::new(&logger, o!("logger" => "SaveConvertedProductUseCase")),
            store_converted_product_service: DynamoDbStoreConvertedProductService { dynamo_db_client: client, table_name: dynamodb_table_name.to_string() },
            notify_update_product_service: KafkaNotifyUpdateProductService { producer, topic_name: topic.to_string() }
        }
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler.handle_lambda_kafka_event(lambda_event).await.expect("TODO: panic message");


    // then

    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client.get_item().table_name(dynamodb_table_name).key("id", AttributeValue::S("p1".to_string())).send().await.expect("DB error");
    assert_eq!("p1", item_output.item().expect("Product not found in database!").get("id").unwrap().as_s().unwrap());
    //assert_eq!("{}", item_output.item().unwrap().get("data").unwrap().as_s().unwrap());
    println!("Converted product: {}", item_output.item().unwrap().get("data").unwrap().as_s().unwrap());

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["UPDATE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    }

}

#[tokio::test]
async fn delete_product_tombstone() {
    let logger = initialize_logger();
    let docker = clients::Cli::default();
    let localstack = docker.run(localstack::Localstack::default());
    let localstack_port = localstack.get_host_port_ipv4(4566);
    let kafka_node = docker.run(kafka::Kafka::default());
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT)
    );


    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;
    let check_client = client.clone();

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        log: Logger::new(&logger, o!("logger" => "LambdaKafkaEventHandler")),
        save_converted_product_use_case: SaveConvertedProductUseCase {
            log: Logger::new(&logger, o!("logger" => "SaveConvertedProductUseCase")),
            store_converted_product_service: DynamoDbStoreConvertedProductService { dynamo_db_client: client, table_name: dynamodb_table_name.to_string() },
            notify_update_product_service: KafkaNotifyUpdateProductService { producer, topic_name: topic.to_string() }
        }
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler.handle_lambda_kafka_event(lambda_event).await.expect("TODO: panic message");


    // then
    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client.get_item().table_name(dynamodb_table_name).key("id", AttributeValue::S("p1".to_string())).send().await.expect("f");
    assert_eq!("p1", item_output.item().expect("Product not found in database!").get("id").unwrap().as_s().unwrap());

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["UPDATE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    }

    println!("Prepare tombstone");
    let lambda_event = create_lambda_event(topic, "p1", &None);

    // when
    println!("Call lambda handler");
    lambda_event_handler.handle_lambda_kafka_event(lambda_event).await.expect("TODO: panic message");

    // then
    //## DynamoDB
    println!("Check result in DynamoDB");
    let item_output = check_client.get_item().table_name(dynamodb_table_name).key("id", AttributeValue::S("p1".to_string())).send().await.expect("f");
    assert_eq!(None, item_output.item());

    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    let expected = vec!["DELETE"];
    for produced in expected {
        let borrowed_message = tokio::time::timeout(Duration::from_secs(10), message_stream.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            produced,
            borrowed_message
                .unwrap()
                .payload_view::<str>()
                .unwrap()
                .unwrap()
        );
    
    }

}

#[tokio::test]
async fn change_detection_save() {
    let logger = initialize_logger();
    let docker = clients::Cli::default();
    let localstack = docker.run(localstack::Localstack::default());
    let localstack_port = localstack.get_host_port_ipv4(4566);
    let kafka_node = docker.run(kafka::Kafka::default());
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_node.get_host_port_ipv4(kafka::KAFKA_PORT)
    );


    // given
    let topic = "test-topic";
    let (producer, consumer) = initialize_kafka(bootstrap_servers, topic);
    let dynamodb_table_name = "test_table";
    let client = initialize_dynamodb(localstack_port, dynamodb_table_name).await;

    let mut testconsumer = consumer.stream();
    let result = tokio::time::timeout(Duration::from_secs(10), testconsumer.next());
    result.await.expect("No messages");
    drop(testconsumer);

    println!("Initialize Lambda handler");
    let lambda_event_handler = LambdaKafkaEventHandler {
        log: Logger::new(&logger, o!("logger" => "LambdaKafkaEventHandler")),
        save_converted_product_use_case: SaveConvertedProductUseCase {
            log: Logger::new(&logger, o!("logger" => "SaveConvertedProductUseCase")),
            store_converted_product_service: DynamoDbStoreConvertedProductService { dynamo_db_client: client, table_name: dynamodb_table_name.to_string() },
            notify_update_product_service: KafkaNotifyUpdateProductService { producer, topic_name: topic.to_string() }
        }
    };

    println!("Prepare test data");
    let lambda_event = create_lambda_event_from_file(topic, "p1", "tests/data/product1.json");

    // when
    println!("Call lambda handler");
    lambda_event_handler.handle_lambda_kafka_event(lambda_event.clone()).await.expect("TODO: panic message");
    lambda_event_handler.handle_lambda_kafka_event(lambda_event).await.expect("TODO: panic message");

    // then
    //## Kafka
    println!("Check result in Kafka topic");
    let mut message_stream = consumer.stream();
    assert_eq!(true, tokio::time::timeout(Duration::from_secs(5), message_stream.next()).await.is_ok());
    assert_eq!(false, tokio::time::timeout(Duration::from_secs(5), message_stream.next()).await.is_ok());

}

fn initialize_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_envlogger::new(drain).fuse();
    //let drain = slog::LevelFilter::new(drain, slog::Level::Info).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = slog::Logger::root(drain, o!());
    slog_scope::set_global_logger(Logger::new(&logger, o!("logger" => "global"))).cancel_reset();
    slog_stdlog::init().unwrap_or_default();
    logger
}

fn initialize_kafka(bootstrap_servers: String, topic: &str) -> (FutureProducer, StreamConsumer) {
    //## Kafka
    println!("Initialize Kafka");

    let producer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer>()
        .expect("Failed to create Kafka FutureProducer");

    let consumer = ClientConfig::new()
        .set("group.id", "testcontainer-rs")
        .set("bootstrap.servers", &bootstrap_servers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create::<StreamConsumer>()
        .expect("Failed to create Kafka StreamConsumer");

    consumer
        .subscribe(&[topic])
        .expect("Failed to subscribe to a topic");

    return (producer, consumer);
}

async fn initialize_dynamodb(localstack_port: u16, dynamodb_table_name: &str) -> Client {
    //## localstack
    println!("Initialize Localstack and DynamoDB");

    let region_provider = RegionProviderChain::default_provider().or_else("eu-central-1");
    let shared_config = aws_config::from_env().region(region_provider).credentials_provider(Credentials::new("example", "example", None, None, "example")).load().await;
    //let shared_config = aws_config::load_from_env().await;
    let mut dynamodb_client_builder = aws_sdk_dynamodb::config::Builder::from(&shared_config);
    dynamodb_client_builder = dynamodb_client_builder.endpoint_url(&format!("http://127.0.0.1:{}/", localstack_port));
    let dynamodb_client = Client::from_conf(dynamodb_client_builder.build());
    let key_name = "id";
    let ks = KeySchemaElement::builder()
        .attribute_name(key_name)
        .key_type(KeyType::Hash)
        .build();
    let ad = AttributeDefinition::builder()
        .attribute_name(key_name)
        .attribute_type(ScalarAttributeType::S)
        .build();
    let pt = ProvisionedThroughput::builder()
        .read_capacity_units(10)
        .write_capacity_units(5)
        .build();
    dynamodb_client.create_table()
        .table_name(dynamodb_table_name)
        .key_schema(ks)
        .attribute_definitions(ad)
        .provisioned_throughput(pt)
        .send().await.expect("Could not create table");

    return dynamodb_client;
}

fn create_lambda_event_from_file(topic: &str, product_id: &str, file_name: &str) -> LambdaEvent<KafkaEvent> {
    let file = fs::File::open(file_name)
        .expect("file should open read only");
    let product: Product = serde_json::from_reader(file)
        .expect("file should be proper JSON");
    create_lambda_event(topic, product_id, &Some(product))
}

fn create_lambda_event(topic: &str, product_id: &str, product_data: &Option<Product>) -> LambdaEvent<KafkaEvent> {
    let mut product = None;
    let mut header: HashMap<String, Vec<u8>> = HashMap::new();
    header.insert(String::from("sampleHeader"), String::from("sampleHeaderValue").into_bytes());
    if product_data.is_some() {
        product = Some(serde_json::to_string(product_data.as_ref().unwrap()).unwrap().encode().unwrap());
    }
    let kafka_record = KafkaRecord {
        topic: Some(topic.to_string()),
        partition: 0,
        offset: 0,
        timestamp: MillisecondTimestamp(DateTime::default()),
        timestamp_type: None,
        key: Some(product_id.to_string().encode().unwrap()),
        value: product,
        headers: vec!(header),
    };
    let kafka_event = KafkaEvent {
        event_source: None,
        event_source_arn: None,
        records: HashMap::from([(topic.to_string(), vec!(kafka_record))]),
        bootstrap_servers: None
    };
    let lambda_event = LambdaEvent {
        payload: kafka_event,
        context: Default::default()
    };

    return lambda_event;
}
