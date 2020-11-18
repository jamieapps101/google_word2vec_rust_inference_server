mod word2vec;
mod server;
use std::path::PathBuf;
use clap::{Arg, App};

fn main() {
    let matches = App::new("word2vec server")
                        .version("1.0")
                        .author("Jamie Apps, jamieapps101@gmail.com")
                        .about("HTTP request LUT using google's word2vec")
                        .arg(Arg::with_name("bin")
                            .short("b")
                            .long("bin")
                            .value_name("FILE")
                            .help("Path to file containing word/vector pairs, usually *.bin")
                            .takes_value(true)
                            .required(true))
                        .arg(Arg::with_name("port")
                            .short("p")
                            .long("port")
                            .value_name("PORT")
                            .help("Port on which to accept http requests, default 3030")
                            .takes_value(true)
                            .required(false))
                        .get_matches();

    let model_path = PathBuf::from(matches.value_of("bin").unwrap());
    let port: u16  = matches.value_of("port").unwrap_or("3030").parse::<u16>().unwrap();
    server::begin(model_path,port);
}
