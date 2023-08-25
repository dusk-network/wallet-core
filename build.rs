// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/schema.json");

    let schema = PathBuf::from("assets/schema.json").canonicalize().unwrap();
    let types = PathBuf::from("src/types.rs");

    schemafy_lib::Generator::builder()
        .with_input_file(&schema)
        .build()
        .generate_to_file(&types)
        .unwrap();

    let types = types.canonicalize().unwrap();
    let contents = fs::read_to_string(&types).unwrap();

    // some limitations of schemafy will not allow it to parse the correct
    // integer type as they incorrectly fallback any integer to `i64`
    let contents = contents.replace("Vec<i64>", "Vec<u8>");
    let contents = contents.replace("i64", "u64");

    let header = r#"// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Arguments and responses to the module requests

// THIS FILE IS AUTO GENERATED!!

#![allow(missing_docs)]

use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};"#;

    let contents = header.to_owned() + &contents;

    fs::write(&types, contents).unwrap();
}
