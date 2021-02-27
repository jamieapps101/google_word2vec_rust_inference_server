use crate::word2vec;
use std::path::PathBuf;
use threadpool::ThreadPool;
use tokio;
use tokio::sync::oneshot;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4, SocketAddr};
use std::sync::Arc;
use std::sync::Mutex;

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
    data: HashMap<String,Option<Vec<f32>>>,
}

#[derive(Deserialize, Serialize)]
struct TestResponse {
    data: String,
}

#[derive(Clone, Debug)]
pub enum ThreadComm {
    Word2Vec(String),
    WordVec(Option<Vec<f32>>),
    Exit,
}

#[derive(Clone)]
pub struct Comm<T> {
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

    pub fn send(&self, item: T) -> Result<(),SendError<T>> {
        self.sender.send(item)
    }
    fn recv(&self) -> Result<T,RecvError> {
        self.receiver.recv()
    }

    fn iter(&self) -> Iter<'_,T> {
        self.receiver.iter()
    }
}

pub struct Server {
    comm_tx:       Comm<ThreadComm>,
    comm_rx:       Comm<ThreadComm>,
    model:         Arc<Mutex<word2vec::Model>>,
    pool:          ThreadPool,
} 

impl Server {
    pub fn init(model_path: PathBuf) -> Option<Server> {
        print!("Loading model... ");
        let model = match word2vec::Model::new(model_path) {
            Ok(model_str) => model_str,
            Err(reason) => {
                println!("{:?}",reason);
                return None;
            }, 
        };
        println!("Done");
        println!("words:{}\nvector size:{}\n",model.total_words, model.size);
        let pool =  ThreadPool::new(1); // one for the model, one for the server
        let (comm_tx,comm_rx):(Comm<ThreadComm>,Comm<ThreadComm>) = Comm::new();
        Some(Server {
            comm_tx,
            comm_rx,
            model:Arc::new(Mutex::new(model)),
            pool,
        })
    }

    pub fn begin(&mut self, port: u16){
        let (http_shutdown_tx, http_server_shutdown_rx): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();
        let comm_b =  self.comm_tx.clone();

        self.pool.execute(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(Self::serve(comm_b,port, http_server_shutdown_rx));
        });
        let local_model = self.model.clone();

        self.infer(local_model);
            // after killing 
        http_shutdown_tx.send(()).unwrap();
    }

    pub fn get_shutdown_tx(&self) -> Comm<ThreadComm> {
        self.comm_tx.clone()
    }

    fn infer(&self, model: Arc<Mutex<word2vec::Model>>) {
        println!("starting inference server");
        // get model out of the Arc/Mutex
        let model = model.as_ref().lock().unwrap();
        for message in self.comm_rx.iter() {
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
                    if let Err(reason) = self.comm_rx.send(ThreadComm::WordVec(return_message)) {
                        println!("Warning, could not send message because:\n\t{}",reason);
                    }
                },
                ThreadComm::Exit => {
                    break;
                },
                _ => {
                    // Do nothing for anothing else
                },
            }
        }
    
        println!("Exiting inference server");
    }
    
    async fn serve(comm:Comm<ThreadComm>, port: u16, shutdown_rx: oneshot::Receiver<()>) {
        let socket = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0,0,0,0),port));
        println!("starting HTTP server on: {}",socket);
        let convert = warp::get()
            .and(warp::path("convert"))
            // Only accept bodies smaller than 1Mb...
            .and(warp::body::content_length_limit(1024*1024))
            .and(warp::body::json())
            .map(move |payload: ConvertPayload| {
                let mut response_map: HashMap<String,Option<Vec<f32>>> = HashMap::with_capacity(payload.words.len());
                for word in payload.words.iter() {
                    let s: String = (*word).clone();
                    if let Err(reason) = comm.send(ThreadComm::Word2Vec(s.clone())) {
                        println!("I errored bc:\n\t{}",reason);
                    }
                    if let ThreadComm::WordVec(vec_response_opt) = comm.recv().unwrap() {
                        response_map.insert(s, vec_response_opt);
                    } else {
                        response_map.insert(s, None);
                    }
                }
                warp::reply::json(&ConvertResponse {
                    data: response_map,
                })
            });
    
            let (_addr, server) = warp::serve(convert).bind_with_graceful_shutdown(socket, async {shutdown_rx.await.ok(); });
            server.await;
            println!("Exiting HTTP server");
    }
}





#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn run_server_small() {
        let short_model_path =PathBuf::from("./test_material/vectors.bin"); 
        let mut serv = Server::init(short_model_path).unwrap();
        serv.begin(3030);
    }

    #[test]
    fn run_server_big() {
        let short_model_path =PathBuf::from("./test_material/GoogleNews-vectors-negative300.bin");
        let mut serv = Server::init(short_model_path).unwrap();
        serv.begin(3030); 
    }
}
