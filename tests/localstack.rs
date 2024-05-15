use std::collections::HashMap;

use testcontainers::core::{Mount, WaitFor};
use testcontainers::Image;

const NAME: &str = "localstack/localstack";
const TAG: &str = "latest";

#[derive(Debug)]
pub struct Localstack {
    env_vars: HashMap<String, String>,
    volumes: Vec<Mount>,
}

impl Default for Localstack {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        let volumes = vec![Mount::bind_mount(
            "/var/run/docker.sock".to_owned(),
            "/var/run/docker.sock".to_owned(),
        )];

        env_vars.insert("SERVICES".to_owned(), "dynamodb".to_owned());
        env_vars.insert("EAGER_SERVICE_LOADING".to_owned(), "1".to_owned());
        env_vars.insert("DEFAULT_REGION".to_owned(), "eu-central-1".to_owned());
        env_vars.insert("PORT_WEB_UI".to_owned(), "9888".to_owned());
        env_vars.insert("LAMBDA_REMOTE_DOCKER".to_owned(), "false".to_owned());
        env_vars.insert("HOST_TMP_FOLDER".to_owned(), "/tmp/localstack".to_owned());

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

    fn mounts(&self) -> Box<dyn Iterator<Item = &Mount> + '_> {
        Box::new(self.volumes.iter())
    }
}
