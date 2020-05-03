extern crate gfcgi;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::thread;

struct Router {
    state: State,
}
struct State {
    file_path: Arc<String>,
}

fn main() {
    pretty_env_logger::init();

    let addr: String = env::var("LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:4128".into())
        .parse()
        .unwrap();
    info!("Listening on address {}", addr);

    let try_path = parse_env("FILEPATH");
    if let Err(_) = try_path {
        panic!("Fatal! Environment variable FILEPATH is invalid");
    }
    let file_path = try_path.unwrap();
    info!("Will list files in path {}", file_path);

    let client = gfcgi::Client::new(addr);
    client.run(Router {
        state: State {
            file_path: Arc::new(file_path),
        },
    });
    thread::park(); // keep main process
}

fn parse_env(param: &str) -> Result<String, std::io::Error> {
    let env_val = env::var(param);
    if let Err(_) = env_val {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    let path = env_val.unwrap().parse::<String>();
    if let Err(_) = path {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    return Ok(path.unwrap());
}

impl gfcgi::Handler for Router {
    fn process(&self, http_pair: &mut gfcgi::HttpPair) {
        let request = http_pair.request();
        // e.g. "SCRIPT_NAME": "/api/any.cgi",
        let script_name = request.header_utf8(b"SCRIPT_NAME").unwrap_or_default();
        debug!("SCRIPT_NAME header {}", script_name);
        let tokens: Vec<&str> = script_name.split("/").collect();

        let script_name = tokens[tokens.len() - 1];
        debug!("Parsed script_name {}", script_name);
        let response = http_pair.response();
        if script_name == "list_file.cgi" {
            list_file(self, response);
            return;
        }
        not_found(response, &format!("Try to find script: {}", script_name))
    }
}

fn list_file(router: &Router, response: &mut gfcgi::Response) {
    let path = router.state.file_path.to_string();
    let mut file_list: Vec<String> = Vec::new();
    let re = regex::Regex::new(r".+\.html?").unwrap();
    match fs::read_dir(path) {
        Ok(iter) => {
            for o_path in iter {
                match o_path {
                    Ok(ok_path) => {
                        let path = ok_path.path();
                        let try_metadata = fs::metadata(&path);
                        if let Ok(metadata) = try_metadata {
                            if metadata.is_file() {
                                let path_str =
                                    ok_path.path().to_str().unwrap_or_default().to_string();
                                let tokens: Vec<&str> = path_str.split("/").collect();
                                let path_str = tokens[tokens.len() - 1];
                                if re.is_match(path_str) {
                                    file_list.push(path_str.to_string());
                                }
                            }
                        }
                    }
                    Err(err) => {
                        error!("IO error while iterate path {:?}", err);
                    }
                }
            }
        }
        Err(err) => {
            error!("IO Read dir error {:?}", err);
            internal_error(response, "can't list files");
        }
    }
    info!("SUCCESS Returns number of files {}", file_list.len());
    let json_result = serde_json::to_string(&file_list);
    send_json(response, json_result.unwrap_or_default());
}

fn send_json(response: &mut gfcgi::Response, body: String) {
    response.status(200);
    response.header_utf8("Content-type", "application/json");
    let bytes = body.as_bytes();
    let len = bytes.len();
    response.header(b"Content-length", len.to_string().as_bytes());
    response.write(body.as_bytes()).expect("send body");
}

fn internal_error(response: &mut gfcgi::Response, msg: &str) {
    response.status(500);
    response.header_utf8("Content-type", "text/plain");
    response.write(msg.as_bytes()).expect("send body");
}

fn not_found(response: &mut gfcgi::Response, msg: &String) {
    warn!("404 to client {}", msg);
    response.status(404);
    response.header_utf8("Content-type", "application/json");

    response
        .write(format!("Script not found. {}", msg).as_bytes())
        .expect("response write error");
}
