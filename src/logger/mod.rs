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

pub mod state;

use std::str;
use std::error::Error;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::time::Duration;
use shuteye::sleep;

use chrono::{DateTime, Utc};
use config::configuration::{ConfigLogFile};
use rusoto_logs::{
    CloudWatchLogs,
    CreateLogGroupRequest,
    CreateLogStreamRequest,
    InputLogEvent,
    PutLogEventsRequest,
    PutLogEventsError,
};

pub fn watch(log_file: ConfigLogFile, client: &Box<CloudWatchLogs>) {
    println!("File: {}", log_file.file);
    println!("Group name: {}", log_file.log_group_name);
    println!("Stream Name: {}", log_file.log_stream_name);
    println!("Datetime: {}", log_file.datetime_format);

    create_group(&log_file.log_group_name, client);
    create_stream(&log_file.log_group_name, &log_file.log_stream_name, client);

    // Infinite loop
    consumer(&log_file, client);
}

/// Consumer is the method used to read from file and
fn consumer(log_file: &ConfigLogFile, client: &Box<CloudWatchLogs>)
{
    // TODO must have the general config to set the custom states_dir
    let states_dir: Option<String> = None;
    let mut token: Option<String> = None;
    let mut offset: u64 = 0;

    match state::load(log_file.file.to_owned(), states_dir.to_owned()) {
        Ok(state) => {
            token = Some(state.token);
            offset = state.offset;
        },
        Err(_) => {},
    }

    loop {
        let mut _offset: u64 = offset;
        let content: String = read_file(&log_file.file, &mut _offset);

        // TODO find log_stream in db state and last sequence_token
        let mut delay = Duration::new(5, 0);

        match put_log_events(
            &content,
            &log_file.log_group_name,
            &log_file.log_stream_name,
            token,
            client
        ) {
            Ok(LogEventResponse) => {
                token = LogEventResponse.token;
                state::save(log_file.file.to_owned(), states_dir.to_owned(), state::State {
                    token: token.to_owned().unwrap(),
                    offset: _offset,
                });

                // Waiter in milliseconds
                delay = Duration::new(0, 200*1000000);
                offset = _offset;
            },
            Err(LogEventError) => {
                token = LogEventError.token;
            },
        }

        // TODO pause of x ms depending of the size of vector
        println!("\n-----------------------------\n");

        sleep(delay);
    }
}

fn create_group(log_group_name: &String, client: &Box<CloudWatchLogs>) {
    let log_group_request: CreateLogGroupRequest = CreateLogGroupRequest {
        log_group_name: log_group_name.to_owned(),
        tags: None,
    };

    let result = client.create_log_group(&log_group_request);

    match result {
        //TODO find how to match only CreateLogStreamError::ResourceAlreadyExists
        Err(why) => println!("The creation of log group have failed: {}", why.description()),
        Ok(_) => println!("Log group {} created with success", log_group_name),
    }
}

fn create_stream(
    log_group_name: &String,
    log_stream_name: &String,
    client: &Box<CloudWatchLogs>
) {
    let log_stream_request: CreateLogStreamRequest = CreateLogStreamRequest {
        log_group_name: log_group_name.to_owned(),
        log_stream_name: log_stream_name.to_owned(),
    };

    let result = client.create_log_stream(&log_stream_request);

    match result {
        Err(why) => println!("The creation of log stream have failed: {}", why.description()),
        Ok(_) => println!("Log stream {} create with success", log_stream_name),
    }
}

/// Read the log file name at specific position
///
/// Return String reads or None if eof reached
/// 
/// The offset is relative to the start of the file and thus independent
/// from the current cursor.
fn read_file(file_name: &String, offset: &mut u64) -> String {
    let path = Path::new(file_name);
    let path_display = path.display();
    let file = match File::open(&path) {
        Err(why) => panic!("ERROR: cannot open logfile {} : {}",
                           path_display, why.description()),
        Ok(file) => file,
    };

    let mut buffer = [0; 1028];
    let mut content: String;

    match file.read_at(&mut buffer, offset.to_owned()) {
        Err(why) => panic!("couldn't read {} : {}", path_display, why.description()),
        Ok(n) => {
            content = str::from_utf8(&buffer[..n]).unwrap().to_string();

            if let Some(line_feed_offset) = content.rfind("\n") {
                content.truncate(line_feed_offset);
            }

            *offset += content.len() as u64;

            println!("the size of content {} are : {}", path_display, n);
            println!("the size of content truncate {} are : {}", path_display, content.len());
            println!("the offset are now at : {}", offset);
            println!("The content of file are : {:?}", content);

            return content;
        }
    };
}

struct LogEventResponse {
    token: Option<String>
}
struct LogEventError {
    token: Option<String>
}

// We should / MUST use state file to persist the position with the latest
// sequence_token to avoid duplication log or data loss if agent restart
// And also maybe replace message by vector to reduce the HTTP call and
// increase performance on high rate log stream.
fn put_log_events(
    message: &String,
    log_group_name: &String,
    log_stream_name: &String,
    token: Option<String>,
    client: &Box<CloudWatchLogs>
) -> Result<LogEventResponse, LogEventError> {
    let utc: DateTime<Utc> = Utc::now();
    let tz_milliseconds: i64 = utc.timestamp() * 1000;
    let mut events: Vec<InputLogEvent> = Vec::new();

    for line in message.lines() {
        if line.is_empty() {
            continue;
        }
        let inline_event: InputLogEvent = InputLogEvent {
            message: line.to_string(),
            timestamp: tz_milliseconds, // TODO must be defined by the beginning of string parse using log_file.datetime_format
        };
        events.push(inline_event);
    }

    if events.is_empty() {
        return Ok(LogEventResponse { token });
    }

    let log_event_request: PutLogEventsRequest = PutLogEventsRequest {
        log_events: events,
        log_group_name: log_group_name.to_owned(),
        log_stream_name: log_stream_name.to_owned(),
        sequence_token: token,
    };

    let result_log = client.put_log_events(&log_event_request);

    return match result_log {
        Err(why) => {
            match why {
                PutLogEventsError::InvalidSequenceToken(cause) => {
                    let pat = "expected sequenceToken is: ";
                    let token = match cause.find(pat) {
                        None => None,
                        Some(position) => {
                            Some(cause.get(pat.len() + position..).unwrap().to_string())
                        },
                    };
                    Err(LogEventError { token })
                },
                _ => {
                    // Err(LogEventError { parent_error: why, token: None })
                    panic!("Put Log event have failed: {}", why.description());
                },
            }
        },
        Ok(response) => {
            let token: String = response.next_sequence_token.unwrap();
            println!("Put Log with success time:{}", tz_milliseconds);
            println!("Next seq token :{}", token);
            Ok(LogEventResponse { token: Some(token) })
        },
    }
}
