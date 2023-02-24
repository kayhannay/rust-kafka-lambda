extern crate serde;
extern crate schemafy_core;

use serde::Deserialize;
use serde::Serialize;

schemafy::schemafy!(
    root: Product
    "schema/product.json"
);
