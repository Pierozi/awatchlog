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

use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::io::prelude::*;
use std::io::BufWriter;
use serde_json;
use sha1;

const DEFAULT_STATES_PATH: &'static str = "/usr/share/awatchlog/states";

#[derive(Serialize, Deserialize)]
pub struct State {
    pub token: String,
    pub offset: u64,
}

pub struct Error {
    pub code: u64,
    pub message: String,
}

pub fn load(logfile: String, states_dir: Option<String>) -> Result<State, Error> {
    let state_path_dir = get_state_file_path(logfile, states_dir);
    let state_path = Path::new(&state_path_dir);

    return match File::open(state_path) {
        Ok(file) => Ok(serde_json::from_reader(file).unwrap()),
        Err(_) => Err(Error {
            code: 0,
            message: "State file not found".to_string()
        })
    };
}

pub fn save(logfile: String, states_dir: Option<String>, state: State) {
    let state_path_dir = get_state_file_path(logfile, states_dir);
    let state_json = json!(state);
    let json: String = state_json.to_string();

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(state_path_dir).unwrap();

    {
        let mut writer = BufWriter::new(file);
        match writer.write_all(json.as_bytes()) {
            Ok(_) => {},
            Err(_) => {
                //TODO put warning in log
            },
        }
    }
}

fn get_state_file_path(logfile: String, state: Option<String>) -> String {
    let state_path_dir: String = if let Some(custom_state_path) = state {
        custom_state_path
    } else {
        DEFAULT_STATES_PATH.to_string()
    };

    let state_path = Path::new(&state_path_dir);

    if false == state_path.exists() {
        println!("States path not exists, try to create at: {}", state_path_dir);
        match fs::create_dir(state_path) {
            Ok(_) => {},
            Err(_) => panic!("Cannot create sates path at {}", state_path_dir)
        }
    }

    let mut file_path_sha1 = sha1::Sha1::new();
    file_path_sha1.update(logfile.as_bytes());

    return format!(
        "{}/{}.json",
        state_path_dir,
        file_path_sha1.digest().to_string()
    );
}