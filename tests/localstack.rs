use std::collections::HashMap;

use testcontainers::core::WaitFor;
use testcontainers::{Image, ImageArgs};

const NAME: &str = "localstack/localstack";
const TAG: &str = "latest";

struct LocalstackArgs {}

impl ImageArgs for LocalstackArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Localstack {
    env_vars: HashMap<String, String>,
    volumes: HashMap<String, String>,
}

impl Default for Localstack {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        let mut volumes = HashMap::new();

        env_vars.insert("SERVICES".to_owned(), "dynamodb".to_owned());
        env_vars.insert("EAGER_SERVICE_LOADING".to_owned(), "1".to_owned());
        env_vars.insert("DEFAULT_REGION".to_owned(), "eu-central-1".to_owned());
        env_vars.insert("PORT_WEB_UI".to_owned(), "9888".to_owned());
        env_vars.insert("LAMBDA_REMOTE_DOCKER".to_owned(), "false".to_owned());
        env_vars.insert("HOST_TMP_FOLDER".to_owned(), "/tmp/localstack".to_owned());

        volumes.insert(
            "/var/run/docker.sock".to_owned(),
            "/var/run/docker.sock".to_owned(),
        );

        Self { env_vars, volumes }
    }
}

impl Image for Localstack {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "Execution of \"preload_services\" took",
        )]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }

    fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.volumes.iter())
    }
}
