extern crate gfcgi;
use std::env;
use std::io::{Write};
use std::thread;
use std::fs;


#[derive(Clone)]
struct Router;
impl Router {
    fn new() -> Self {
        Router {}
    }
}
fn main() {
    let addr: String = env::var("LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:4128".into())
        .parse()
        .expect("can't parse LISTEN variable");


    let client = gfcgi::Client::new(addr);
    client.run(Router::new());
    if cfg!(feature = "spawn") {
        client.run(Router::new()); // spawn worker
    }
    thread::park(); // keep main process
}

impl gfcgi::Handler for Router {
    fn process(&self, http_pair: &mut gfcgi::HttpPair) {
        let request = http_pair.request();
        // e.g. "SCRIPT_NAME": "/api/any.cgi",
        let script_name = request.header_utf8(b"SCRIPT_NAME").unwrap_or_default();
        let tokens: Vec<&str> = script_name.split("/").collect();

        let script_name = tokens[tokens.len() - 1];
        let response = http_pair.response();
        if script_name == "list_file.cgi" {
            list_file(response);
            return;
        }
        not_found(response)
    }
}

fn list_file(response: &mut gfcgi::Response) {
    let path: String = env::var("FILEPATH").unwrap().parse().expect("can't parse FILEPATH");
    let mut file_list: Vec<String> = Vec::new();
    let re = regex::Regex::new(r".+\.html?").unwrap();
    match fs::read_dir(path) {
        Ok(iter)=>{
            for o_path in iter {
                match o_path {
                    Ok(ok_path) => {
                        let path = ok_path.path();
                        let metadata = fs::metadata(&path).unwrap();
                        if metadata.is_file() {
                            let path_str =  ok_path.path().to_str().unwrap_or_default().to_string();
                            let tokens: Vec<&str> = path_str.split("/").collect();
                            let path_str = tokens[tokens.len()-1];
                            if re.is_match(path_str) {
                                file_list.push(path_str.to_string());
                            }
                        }

                    },
                    _ => {}
                }
            }
        },
        Err(_)=>{
            internal_error(response, "can't list files");
        }
    }
    let json_result = serde_json::to_string(&file_list);
    send_json(response, json_result.unwrap_or_default());
}

fn send_json(response: &mut gfcgi::Response, body:String ){
    response.status(200);
    response.header_utf8("Content-type", "application/json");
    // response.write("\n\n".as_bytes()).expect("send body");
    response.write(body.as_bytes()).expect("send body");
}


fn internal_error(response: &mut gfcgi::Response,  msg: &str){
    response.status(500);
    response.header_utf8("Content-type", "text/plain");
    response.write(msg.as_bytes()).expect("send body");
}

fn not_found(response: &mut gfcgi::Response) {
    response.status(404);
    response.header_utf8("Content-type", "application/json");
    response
        .write(b"Not found! available script: list_file.cgi")
        .expect("response write error");
}
