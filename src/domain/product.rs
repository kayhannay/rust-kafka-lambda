extern crate schemafy_core;
extern crate serde;

use serde::Deserialize;
use serde::Serialize;

schemafy::schemafy!(
    root: Product
    "schema/product.json"
);
