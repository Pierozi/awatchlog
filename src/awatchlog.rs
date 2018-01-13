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

#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate shuteye;

extern crate rusoto_credential;
extern crate rusoto_logs;
extern crate rusoto_core;

use std::str::FromStr;
use rusoto_credential::{DefaultCredentialsProvider};
use rusoto_core::{default_tls_client, Region};
use rusoto_logs::{
    CloudWatchLogs,
    CloudWatchLogsClient,
};

mod logger;
mod config;
use config::configuration;
use config::configuration::{AwatchLogConfig};
use config::credentials;

pub fn run(config_file: Option<String>, credentials_file: Option<String>) {
    let config: AwatchLogConfig = configuration::parse(config_file);

    // TODO must auto detect region by using instance metadata
    // use config::discovery::metadata

    let region = match Region::from_str(&config.general.region) {
        Ok(region) => region,
        Err(_) => Region::UsEast1,
    };

    match credentials::parse(credentials_file) {
        None => {
            let credentials = DefaultCredentialsProvider::new().unwrap();
            let client = CloudWatchLogsClient::new(
                default_tls_client().unwrap(),
                credentials,
                region
            );
            worker(config, client);
        },
        Some(credentials) => {
            let client = CloudWatchLogsClient::new(
                default_tls_client().unwrap(),
                credentials,
                region
            );
            worker(config, client);
        },
    }
}

fn worker<C: CloudWatchLogs>(config: AwatchLogConfig, client: C) {
    // TODO check if pid already up
    println!("PID FILE: {}", config.general.pid_file);

    // TODO loop over config.logfile
    // Must thread this loop to watch multiple file in same times
    for logfile in config.logfile {
        logger::watch(logfile, &client);
    }
}