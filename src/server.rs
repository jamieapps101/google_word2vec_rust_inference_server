use crate::word2vec;
use std::path::PathBuf;

use warp::Filter;
// use serde_derive::{Deserialize, Serialize};
use serde::{Deserialize, Serialize};

pub struct Server {
    model: word2vec::Model,
}

#[derive(Deserialize, Serialize)]
struct ConvertPayload {
    words: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct ConvertResponse {
    words: Vec<Vec<f32>>,
}

#[derive(Deserialize, Serialize)]
struct TestResponse {
    data: String,
}

impl Server{
    pub fn new(model_path: PathBuf) -> Option<Server> {
        let model = match word2vec::Model::new(model_path) {
            Ok(model_str) => model_str,
            Err(reason) => {
                println!("{:?}",reason);
                return None;
            }, 
        };
        Some(Server {
            model,
        })
    }

    pub async fn begin(&self) {
        let convert = warp::get()
            .and(warp::path("convert"))
            // Only accept bodies smaller than 16kb...
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::json())
            .map(|payload: TestResponse| {
            // .map(|| {
                println!("Got one");
                warp::reply::json(&TestResponse {
                    data: String::from("hello back"),
                })
                // warp::reply::reply()

                // "hello there!"
            });

        warp::serve(convert).run(([127, 0, 0, 1], 3030)).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[ignore]
    fn init_server() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin");
        let server = Server::new(short_model_path); 
    }

    #[tokio::test]
    async fn run_server() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin");
        let server = Server::new(short_model_path).unwrap(); 
        server.begin().await;
    }


}
