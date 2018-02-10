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
use std::collections::HashMap;

use shuteye::sleep;
use regex::Regex;

use chrono::{DateTime, Utc, FixedOffset};
use config::configuration::{ConfigLogFile};
use rusoto_logs::{
    CloudWatchLogs,
    CreateLogGroupRequest,
    CreateLogStreamRequest,
    InputLogEvent,
    PutLogEventsRequest,
    PutLogEventsError,
};

const AWS_MAX_BATCH_SIZE: u64 = 788576; // 1048576 - (10000 * 26)
const AWS_MAX_BATCH_EVENTS: u64 = 10000;
const MIN_BUFFER_SIZE: u64 = 8092;

pub fn watch(log_file: ConfigLogFile, client: &Box<CloudWatchLogs>) {
    println!("File: {}", log_file.file);
    println!("Group name: {}", log_file.log_group_name);
    println!("Stream Name: {}", log_file.log_stream_name);
    println!("Datetime: {:?}", log_file.datetime_format);

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
    let mut buffer_size: u64 = MIN_BUFFER_SIZE;

    match state::load(log_file.file.to_owned(), states_dir.to_owned()) {
        Ok(state) => {
            token = Some(state.token);
            offset = state.offset;
        },
        Err(e) => {
            if e.code != 0 {
                panic!("TODO Error unexpected... {}", e.message);
            }
        },
    }

    loop {
        let mut _offset: u64 = offset;
        let buf_size = buffer_size;
        let mut delay = Duration::new(5, 0);
        println!("the offset are : {}", _offset);
        let content: String = read_file(&log_file.file, &mut _offset, buf_size);

        {
            let delta: u64 = log_file.delta.unwrap_or(512 as u64);
            let content_size = content.len() as u64;
            println!("the content size are : {}", content_size);

            // Wait and continue loop if message empty
            if 0 == content_size {
                println!("Nothing to read at offset {}", offset);
                sleep(delay);
                continue;
            }

            // Ensure the number of lines does not reach 10K limit
            if AWS_MAX_BATCH_EVENTS <= (content.lines().size_hint().0 as u64) {
                // Divide by 2 in order reduce drastically the size
                // and re scale-up progressively if needs
                buffer_size = buffer_size / 2;
                // Ensure buffer size is not lower than MIN_BUFFER_SIZE
                if buffer_size < MIN_BUFFER_SIZE {
                    buffer_size = MIN_BUFFER_SIZE;
                }
                continue;
            }

            // Reduce buffer size if lower than expected (included delta because can be truncated)
            if content_size > delta && content_size < (buffer_size - delta) {
                buffer_size = content_size;
            } else {
                // Otherwise increase buffer size by 50%
                buffer_size = content_size * 150 / 100;

                // Ensure to not allocate more than Max batch size
                if AWS_MAX_BATCH_SIZE < buffer_size {
                    buffer_size = AWS_MAX_BATCH_SIZE;
                }
            }

            // Ensure buffer size is not lower than MIN_BUFFER_SIZE
            if buffer_size < MIN_BUFFER_SIZE {
                buffer_size = MIN_BUFFER_SIZE;
            }
        }

        println!("the buffer size are : {}", buf_size);
        println!("the next buffer size are : {}", buffer_size);

        match put_log_events(
            &content,
            log_file.to_owned(),
            token,
            client
        ) {
            Ok(log_event_response) => {
                token = log_event_response.token;
                state::save(log_file.file.to_owned(), states_dir.to_owned(), state::State {
                    token: token.to_owned().unwrap(),
                    offset: _offset,
                });

                // Waiter in milliseconds
                delay = Duration::new(0, 400*1000000);
                offset = _offset;
            },
            Err(log_event_error) => {
                token = log_event_error.token;
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
fn read_file(file_name: &String, offset: &mut u64, buf_size: u64) -> String {
    let path = Path::new(file_name);
    let path_display = path.display();
    let file = match File::open(&path) {
        Err(why) => panic!("ERROR: cannot open logfile {} : {}",
                           path_display, why.description()),
        Ok(file) => file,
    };

    fn new_buffer(size: u64) -> Vec<u8> {
        vec![0; size as usize]
    }

    let mut buf_sized = new_buffer(buf_size);
    let mut buffer = buf_sized.as_mut_slice();
    let mut content: String;

    match file.read_at(&mut buffer, offset.to_owned()) {
        Err(why) => panic!("couldn't read {} : {}", path_display, why.description()),
        Ok(n) => {
            content = str::from_utf8(&buffer[..n]).unwrap().to_string();

            if let Some(line_feed_offset) = content.rfind("\n") {
                content.truncate(line_feed_offset);
            }

            *offset += content.len() as u64;

            /*println!("the size of content {} are : {}", path_display, n);
            println!("the size of content truncate {} are : {}", path_display, content.len());*/
            //println!("the offset are now at : {}", offset);
            //println!("The content of file are : {:?}", content);

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

fn put_log_events(
    message: &String,
    log_file: &ConfigLogFile,
    token: Option<String>,
    client: &Box<CloudWatchLogs>
) -> Result<LogEventResponse, LogEventError> {
    let mut events: Vec<InputLogEvent> = Vec::new();

    for line in message.lines() {
        if line.is_empty() {
            continue;
        }
        let custom_fmt: Option<String> = log_file.datetime_format.to_owned();
        let tz_milliseconds = match find_timestamp_ms_in_str(line, custom_fmt) {
            None => {
                //TODO WARNING IN LOGGER
                let utc: DateTime<Utc> = Utc::now();
                utc.timestamp() * 1000
            },
            Some(tz_ms) => tz_ms,
        };
        let inline_event: InputLogEvent = InputLogEvent {
            message: line.to_string(),
            timestamp: tz_milliseconds,
        };
        events.push(inline_event);
    }

    if events.is_empty() {
        return Ok(LogEventResponse { token });
    }

    println!("Batch lines: {}", events.len());

    let log_event_request: PutLogEventsRequest = PutLogEventsRequest {
        log_events: events,
        log_group_name: log_file.log_group_name.to_owned(),
        log_stream_name: log_file.log_stream_name.to_owned(),
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
            /*println!("Put Log with success time:{}", tz_milliseconds);
            println!("Next seq token :{}", token);*/
            Ok(LogEventResponse { token: Some(token) })
        },
    }
}

fn find_timestamp_ms_in_str(message: &str, format: Option<String>) -> Option<i64> {
    // Try to extract date using custom format
    let mut fmt: String = String::new();
    let mut date: String = match format {
        None => String::new(),
        Some(custom_format) => {
            fmt = custom_format;
            match extract_datetime(message, &fmt) {
                None => String::new(),
                Some(date_extracted) => date_extracted,
            }
        },
    };

    // Try to extract date with popular datetime format
    if date.is_empty() {
        for fmt_item in list_common_fmt() {
            fmt = fmt_item;
            if let Some(date_extracted) = extract_datetime(message, &fmt) {
                date = date_extracted;
                break;
            }
        }
    }

    if date.is_empty() || fmt.is_empty() {
        return None;
    }

    if let None = fmt.find("%Y") {
        let year = Utc::now().format("%Y").to_string();
        date = format!("{} {}", year.as_str(), date);
        fmt = format!("%Y {}", fmt);
    }

    if let None = fmt.find("%z") {
        let tz: String = FixedOffset::east(1*60*60).to_string();
        date = format!("{} {}", date, tz.as_str());
        fmt = format!("{} %:z", fmt);
    }

    return match DateTime::parse_from_str(&date, &fmt) {
        Ok(datetime) => {
            Some(datetime.timestamp() * 1000)
        },
        Err(_) => None
    };
}

fn extract_datetime(message: &str, fmt: &str) -> Option<String> {
    let mut mapping_fmt: HashMap<&str, &str> = HashMap::new();

    mapping_fmt.insert("%Y", "\\d{4}");
    mapping_fmt.insert("%C", "\\d{2}");
    mapping_fmt.insert("%y", "\\d{2}");
    mapping_fmt.insert("%m", "\\d{2}");
    mapping_fmt.insert("%b", "\\w{3}");
    mapping_fmt.insert("%B", "\\w{4}");
    mapping_fmt.insert("%h", "\\w{3}");
    mapping_fmt.insert("%d", "\\d{2}");
    mapping_fmt.insert("%e", "\\s?\\d");
    mapping_fmt.insert("%a", "\\w{3}");
    mapping_fmt.insert("%A", "\\w{3, 10}");
    mapping_fmt.insert("%w", "\\d");
    mapping_fmt.insert("%u", "\\d");
    mapping_fmt.insert("%U", "\\d{2}");
    mapping_fmt.insert("%W", "\\d{2}");
    mapping_fmt.insert("%G", "\\d{4}");
    mapping_fmt.insert("%g", "\\d{2}");
    mapping_fmt.insert("%V", "\\d{2}");
    mapping_fmt.insert("%j", "\\d{3}");
    mapping_fmt.insert("%D", "\\d{2}/\\d{2}/\\d{2}");
    mapping_fmt.insert("%x", "\\d{2}/\\d{2}/\\d{2}");
    mapping_fmt.insert("%F", "\\d{4}-\\d{2}-\\d{2}");
    mapping_fmt.insert("%v", "\\d{2}-\\w{3}-\\d{4}");
    mapping_fmt.insert("%H", "\\d{2}");
    mapping_fmt.insert("%k", "\\s?\\d");
    mapping_fmt.insert("%I", "\\d{2}");
    mapping_fmt.insert("%l", "\\s?\\d");
    mapping_fmt.insert("%P", "(am|pm)");
    mapping_fmt.insert("%p", "(AM|PM)");
    mapping_fmt.insert("%M", "\\d{2}");
    mapping_fmt.insert("%S", "\\d{2}");
    mapping_fmt.insert("%f", "\\d{9}");
    mapping_fmt.insert("%.f", "\\.\\d{6}");
    mapping_fmt.insert("%.3f", "\\.\\d{3}");
    mapping_fmt.insert("%.6f", "\\.\\d{6}");
    mapping_fmt.insert("%.9f", "\\.\\d{9}");
    mapping_fmt.insert("%R", "\\d{2}:\\d{2}");
    mapping_fmt.insert("%T", "\\d{2}:\\d{2}:\\d{2}");
    mapping_fmt.insert("%X", "\\d{2}:\\d{2}:\\d{2}");
    mapping_fmt.insert("%r", "\\d{2}:\\d{2}:\\d{2} (AM|PM)");
    mapping_fmt.insert("%Z", "\\w{3,8}");
    mapping_fmt.insert("%z", "\\+\\d{4}");
    mapping_fmt.insert("%:z", "\\+\\d{2}:\\d{2}");
    mapping_fmt.insert("%c", "\\w{3} \\w{3} \\s?\\d \\d{2}:\\d{2}:\\d{2} \\d{Y}");
    mapping_fmt.insert("%+", "\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}\\.\\d{6}\\+\\d{2}:\\d{2}");
    mapping_fmt.insert("%s", "\\d{9}");
    mapping_fmt.insert("%t", "\t");
    mapping_fmt.insert("%%", "%");

    let mut fmt_pattern: String = fmt.to_string();

    for (key, value) in mapping_fmt.iter() {
        fmt_pattern = fmt_pattern.replace(key, value);
    }

    let pattern: String = format!(r"(?i)({})", fmt_pattern);
    let regex: Regex = Regex::new(&pattern).unwrap();

    match regex.captures(message) {
        None => None,
        Some(cap) => Some(cap[0].to_string()),
    }
}

fn list_common_fmt() -> Vec<String> {
    vec![
        "%b %d %H:%M:%S".to_string(), // Syslog
        "%Y/%m/%d %H:%M:%S".to_string(), // Nginx Error
        "%a %b %d %H:%M:%S %Y".to_string(), // Apache Error
        "%d/%b/%Y:%H:%M:%S %z".to_string(), // Access log (nginx / apache)
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_find_timestamp_ms_in_str() {
        // Syslog
        assert_eq!(find_timestamp_ms_in_str("Jan 01 16:34:44", Some("%b %d %H:%M:%S".to_string())).unwrap(), 1514820884000 as i64);
        // Nginx Error log
        assert_eq!(find_timestamp_ms_in_str("2018/02/08 13:08:48", Some("%Y/%m/%d %H:%M:%S".to_string())).unwrap(), 1518091728000 as i64);
        // Nginx Access log
        assert_eq!(find_timestamp_ms_in_str("08/Feb/2018:11:18:16 +0100", Some("%d/%b/%Y:%H:%M:%S %z".to_string())).unwrap(), 1518085096000 as i64);
        // Apache Error log
        let apache_error_payload = "[Thu Feb 08 14:32:52 2018] [error] [client 127.0.0.1] client denied by server configuration: /export/home/live/ap/htdocs/test";
        assert_eq!(find_timestamp_ms_in_str(apache_error_payload, Some("%a %b %d %H:%M:%S %Y".to_string())).unwrap(), 1518096772000 as i64);

        let nginx_access_payload = "198.50.136.9 - - [08/Feb/2018:11:18:16 +0100] \"GET /w00tw00t.at.ISC.SANS.DFind:) HTTP/1.1\" 400 166 \"-\" \"-\"";
        assert_eq!(find_timestamp_ms_in_str(nginx_access_payload, Some("%d/%b/%Y:%H:%M:%S %z".to_string())).unwrap(), 1518085096000 as i64);

        assert_eq!(find_timestamp_ms_in_str("08/Feb/2018:11:18:16 +0100", None).unwrap(), 1518085096000 as i64);
    }

    #[test]
    fn case_extract_datetime() {
        let syslog_payload = "Feb  8 06:30:01 plab /USR/SBIN/CRON[23569]: (root) CMD (  /usr/local/bin/fritzcron)";
        assert_eq!(extract_datetime(&syslog_payload, "%b %e %H:%M:%S").unwrap(), "Feb  8 06:30:01".to_string());

        let nginx_error_payload = "2018/02/08 08:28:27 [error] 398#0: *1150116 open() \"/var/www/default/ccvv\" failed (2: No such file or directory), client: 1.0.1.0, server: _, request: \"GET /ccvv HTTP/1.1\", host: \0.1.0.1\"";
        assert_eq!(extract_datetime(&nginx_error_payload, "%Y/%m/%d %H:%M:%S").unwrap(), "2018/02/08 08:28:27".to_string());

        let nginx_access_payload = "198.50.136.9 - - [08/Feb/2018:08:12:58 +0100] \"GET /w00tw00t.at.ISC.SANS.DFind:) HTTP/1.1\" 400 166 \"-\" \"-\"";
        assert_eq!(extract_datetime(&nginx_access_payload, "%d/%b/%Y:%H:%M:%S %z").unwrap(), "08/Feb/2018:08:12:58 +0100".to_string());

        let apache_error_payload = "[Thu Feb 08 14:32:52 2018] [error] [client 127.0.0.1] client denied by server configuration: /export/home/live/ap/htdocs/test";
        assert_eq!(extract_datetime(&apache_error_payload, "%a %b %d %H:%M:%S %Y").unwrap(), "Thu Feb 08 14:32:52 2018".to_string());
    }
}