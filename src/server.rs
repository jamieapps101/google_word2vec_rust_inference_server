use crate::word2vec;
use std::path::PathBuf;
use threadpool::ThreadPool;
pub use tokio;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4, SocketAddr};

use warp::Filter;
// use serde_derive::{Deserialize, Serialize};
use serde::{Deserialize, Serialize};
use crossbeam::channel::{
    unbounded,
    Sender,
    Receiver,
    SendError, 
    RecvError,
    // TryIter,
    // TryRecvError,
    Iter};

#[derive(Deserialize, Serialize)]
struct ConvertPayload {
    words: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct ConvertResponse {
    data: HashMap<String,Vec<f32>>,
}

#[derive(Deserialize, Serialize)]
struct TestResponse {
    data: String,
}

#[derive(Clone, Debug)]
enum ThreadComm {
    Word2Vec(String),
    WordVec(Option<Vec<f32>>),
    // Exit,
}

#[derive(Clone)]
struct Comm<T> {
    sender:Sender<T>,
    receiver:Receiver<T>,
}

impl<T> Comm<T> {
    fn new() -> (Comm<T>,Comm<T>) {
        let (s_a, r_b) = unbounded();
        let (s_b, r_a) = unbounded();

        (
            Comm{
                sender:s_a,
                receiver:r_a,
            },
            Comm{
                sender:s_b,
                receiver:r_b,
            }
        )
    }

    fn send(&self, item: T) -> Result<(),SendError<T>> {
        self.sender.send(item)
    }
    fn recv(&self) -> Result<T,RecvError> {
        self.receiver.recv()
    }

    fn iter(&self) -> Iter<'_,T> {
        self.receiver.iter()
    }
}

pub fn begin(model_path: PathBuf, port: u16) {
    println!("Beginning HTTP server");
    let pool =  ThreadPool::new(1); // one for the model, one for the server

    let (comm_a,comm_b):(Comm<ThreadComm>,Comm<ThreadComm>) = Comm::new();
    print!("Loading model... ");
    let model = match word2vec::Model::new(model_path) {
        Ok(model_str) => model_str,
        Err(reason) => {
            println!("{:?}",reason);
            return ();
        }, 
    };
    pool.execute(move || {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(serve(comm_b,port));
    });
    infer(comm_a,&model);
    println!("Exiting");
}

fn infer(comm:Comm<ThreadComm>,model: &word2vec::Model) {
    println!("starting inference server");
    for message in comm.iter() {
        match message {
            ThreadComm::Word2Vec(word) => {
                let return_message = match model.word2vec(&word) {
                    Some(vector) => {
                        Some((*vector).clone())
                    }
                    None => {
                        None
                    }
                };
                if let Err(reason) = comm.send(ThreadComm::WordVec(return_message)) {
                    println!("Warning, could not send message because:\n\t{}",reason);
                }
            }
            _ => {
                // Do nothing for anothing else
            },
        }
    }

    println!("Exiting inference server");
}

async fn serve(comm:Comm<ThreadComm>, port: u16) {
    let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0,0,0,0),port));
    println!("starting web server on: {}",socket);
    let convert = warp::get()
        .and(warp::path("convert"))
        // Only accept bodies smaller than 1Mb...
        .and(warp::body::content_length_limit(1024*1024))
        .and(warp::body::json())
        .map(move |payload: ConvertPayload| {
            // let mut response_vector: Vec<Vec<f32>> = Vec::with_capacity(payload.words.len());
            let mut response_map: HashMap<String,Vec<f32>> = HashMap::with_capacity(payload.words.len());
            for word in payload.words.iter() {
                let s: String = (*word).clone();
                if let Err(reason) = comm.send(ThreadComm::Word2Vec(s.clone())) {
                    println!("I errored be:\n\t{}",reason);
                }
                if let ThreadComm::WordVec(vec_response_opt) = comm.recv().unwrap() {
                    if let Some(vec_response) = vec_response_opt {
                        // response_vector.push(vec_response);
                        response_map.insert(s, vec_response);
                    }
                }
            }
            warp::reply::json(&ConvertResponse {
                data: response_map,
            })
        });

    warp::serve(convert).run(socket).await;
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn run_server_small() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin"); 
        begin(short_model_path,3030);
    }

    #[test]
    fn run_server_big() {
        let short_model_path =PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin");
        begin(short_model_path,3030); 
    }
}
