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

use std::path::Path;
use config;

const DEFAULT_CONFIG_PATH: &'static str = "/usr/share/awatchlog/config.toml";

#[derive(Deserialize)]
pub struct ConfigGeneral {
    pub pid_file: String,
    pub state_path: String,
    pub region: String,
}

#[derive(Deserialize)]
pub struct ConfigLogFile {
    pub file: String,
    pub log_group_name: String,
    pub log_stream_name: String,
    pub datetime_format: String,
}

#[derive(Deserialize)]
pub struct AwatchLogConfig {
    pub general: ConfigGeneral,
    pub logfile: Vec<ConfigLogFile>,
}

pub fn parse(file: Option<String>) -> AwatchLogConfig {
    let path: String = if let Some(file_path) = file {
        file_path
    } else {
        let default_path = Path::new(&DEFAULT_CONFIG_PATH);

        if false == default_path.exists() {
            panic!("No configuration file found in default path {}\nYou can specify path using -c option", DEFAULT_CONFIG_PATH);
        }

        DEFAULT_CONFIG_PATH.to_string()
    };

    let content = config::parser::get_file_content(path);
    return toml::from_str(&content).unwrap();
}
