// Package: AWatchLog
//
// BSD 3-Clause License
//
// Copyright (c) 2018, Pierre Tomasina
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// * Redistributions of source code must retain the above copyright notice, this
// list of conditions and the following disclaimer.
//
// * Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// * Neither the name of the copyright holder nor the names of its
// contributors may be used to endorse or promote products derived from
// this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

extern crate toml;
extern crate rusoto_credential;

use config;
use std::path::Path;
use rusoto_credential::{StaticProvider};

const DEFAULT_CREDENTIALS_PATH: &'static str = "/etc/awatchlog/credentials.toml";

#[derive(Deserialize)]
struct AwatchLogCredentials {
    aws_access_key_id: String,
    aws_secret_access_key: String,
}

// Parse credentials file
pub fn parse(file: Option<String>) -> Option<StaticProvider> {
    let credentials_content: String;

    match file {
        None => {
            let default_path = Path::new(&DEFAULT_CREDENTIALS_PATH);
            if default_path.exists() {
                credentials_content = config::parser::get_file_content(DEFAULT_CREDENTIALS_PATH.to_owned());
            } else {
                return None;
            }
        },
        Some(file_path) => {
            credentials_content = config::parser::get_file_content(file_path);
        },
    }

    let credentials: AwatchLogCredentials = toml::from_str(&credentials_content).unwrap();

    return Some(StaticProvider::new(
        credentials.aws_access_key_id,
        credentials.aws_secret_access_key,
        None,
        None,
    ));
}
