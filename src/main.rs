mod word2vec;
mod server;
use std::path::PathBuf;

fn main() {
    println!("Hello, world!");
    let short_model_path =PathBuf::from("./test_material/vectors.bin");
    let server = server::Server::new(short_model_path).unwrap();
    server.begin();
}
